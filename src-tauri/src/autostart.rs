//! "Start with Windows": a `HKCU\...\CurrentVersion\Run` entry pointing at this executable.
//!
//! Replaces tauri-plugin-autostart, which wrote an unquoted path (this app installs under
//! `%LOCALAPPDATA%\WinNUT Rust Client\`, i.e. a path with spaces), did not verify the write
//! and gave the UI no way to tell *why* autostart was off. This module:
//!
//! * writes a quoted command line: `"C:\...\ups-client.exe" --autostart`,
//! * reads the registry back after every change and fails loudly if it does not match,
//! * reports when the entry was disabled in Task Manager / Settings › Apps › Startup
//!   (Explorer's `StartupApproved\Run` flag) or points at an old install location,
//! * logs every step to the application log.
#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use serde::Serialize;
use std::path::Path;

/// Registry value name (same as the old plugin used, so its entries are replaced).
pub const ENTRY_NAME: &str = "WinNUT Rust Client";
/// Passed on the command line when Windows starts the app.
pub const AUTOSTART_ARG: &str = "--autostart";

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    /// `false` on platforms other than Windows.
    pub supported: bool,
    /// The Run entry exists, points at this executable and is not disabled by the user.
    pub enabled: bool,
    /// The Run entry exists but starts another path (e.g. an older install location).
    pub stale_path: bool,
    /// The entry was disabled in Task Manager / Settings › Apps › Startup.
    pub disabled_by_user: bool,
    /// The command line currently stored in the Run key, if any.
    pub command: Option<String>,
}

/// The command line written to the Run key.
pub fn expected_command(exe: &Path) -> String {
    format!("\"{}\" {}", exe.display(), AUTOSTART_ARG)
}

/// Extracts the program path from a Run command line (quoted or not).
pub fn command_program(command: &str) -> &str {
    let c = command.trim();
    if let Some(rest) = c.strip_prefix('"') {
        return rest.split('"').next().unwrap_or("");
    }
    // Unquoted (as written by the old plugin): everything up to the first " --" argument.
    match c.find(" --") {
        Some(i) => c[..i].trim_end(),
        None => c,
    }
}

/// `true` if the Run command starts `exe` (Windows paths compare case-insensitively).
pub fn command_targets(command: &str, exe: &Path) -> bool {
    let normalize = |s: &str| s.replace('/', "\\").to_lowercase();
    normalize(command_program(command)) == normalize(&exe.display().to_string())
}

/// Explorer's `StartupApproved\Run` value: the first byte is even (02 / 06) when the entry is
/// enabled and odd (03 / 07) when the user disabled it.
pub fn startup_approved_disabled(bytes: &[u8]) -> bool {
    bytes.first().is_some_and(|b| b & 1 == 1)
}

/// Value written to `StartupApproved\Run` to mark the entry as enabled.
pub const STARTUP_APPROVED_ENABLED: [u8; 12] = [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub fn evaluate(command: Option<String>, approved: Option<&[u8]>, exe: &Path) -> AutostartStatus {
    let points_here = command.as_deref().is_some_and(|c| command_targets(c, exe));
    let disabled = command.is_some() && approved.is_some_and(startup_approved_disabled);
    AutostartStatus {
        supported: true,
        enabled: points_here && !disabled,
        stale_path: command.is_some() && !points_here,
        disabled_by_user: disabled,
        command,
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use log::{info, warn};
    use windows::core::HSTRING;
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_BINARY,
        REG_ROUTINE_FLAGS, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn is_not_found(e: &windows::core::Error) -> bool {
        e.code() == ERROR_FILE_NOT_FOUND.to_hresult()
    }

    /// Reads a value; `Ok(None)` if the key or value does not exist.
    fn get(subkey: &str, name: &str, flags: REG_ROUTINE_FLAGS) -> Result<Option<Vec<u8>>, String> {
        let (sk, nm) = (HSTRING::from(subkey), HSTRING::from(name));
        let mut size: u32 = 0;
        if let Err(e) = unsafe { RegGetValueW(HKEY_CURRENT_USER, &sk, &nm, flags, None, None, Some(&mut size)) } {
            return if is_not_found(&e) { Ok(None) } else { Err(format!("read HKCU\\{subkey}\\{name}: {e}")) };
        }
        let mut buf = vec![0u8; size as usize];
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &sk,
                &nm,
                flags,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        }
        .map_err(|e| format!("read HKCU\\{subkey}\\{name}: {e}"))?;
        buf.truncate(size as usize);
        Ok(Some(buf))
    }

    fn get_string(subkey: &str, name: &str) -> Result<Option<String>, String> {
        Ok(get(subkey, name, RRF_RT_REG_SZ)?.map(|bytes| {
            let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&wide).trim_end_matches('\0').to_string()
        }))
    }

    fn set(subkey: &str, name: &str, kind: u32, data: &[u8]) -> Result<(), String> {
        let (sk, nm) = (HSTRING::from(subkey), HSTRING::from(name));
        unsafe {
            RegSetKeyValueW(HKEY_CURRENT_USER, &sk, &nm, kind, Some(data.as_ptr().cast()), data.len() as u32)
        }
        .map_err(|e| format!("write HKCU\\{subkey}\\{name}: {e}"))
    }

    fn delete(subkey: &str, name: &str) -> Result<(), String> {
        let (sk, nm) = (HSTRING::from(subkey), HSTRING::from(name));
        match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &sk, &nm) } {
            Ok(()) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()),
            Err(e) => Err(format!("delete HKCU\\{subkey}\\{name}: {e}")),
        }
    }

    pub fn status() -> Result<AutostartStatus, String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        let command = get_string(RUN, ENTRY_NAME)?;
        let approved = get(APPROVED, ENTRY_NAME, RRF_RT_REG_BINARY)?;
        Ok(evaluate(command, approved.as_deref(), &exe))
    }

    pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        if enabled {
            let command = expected_command(&exe);
            info!("autostart: writing Run entry {command}");
            let wide: Vec<u8> = command.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();
            set(RUN, ENTRY_NAME, REG_SZ.0, &wide)?;
            // Clear a "disabled" flag left by Task Manager / Settings › Apps › Startup;
            // the user just asked for autostart in the app.
            set(APPROVED, ENTRY_NAME, REG_BINARY.0, &STARTUP_APPROVED_ENABLED)?;
        } else {
            info!("autostart: removing Run entry");
            delete(RUN, ENTRY_NAME)?;
            delete(APPROVED, ENTRY_NAME)?;
        }
        let now = status()?;
        if now.enabled != enabled {
            warn!("autostart: verification failed, registry now reads {now:?}");
            return Err(format!(
                "the registry did not keep the change (Run entry: {})",
                now.command.as_deref().unwrap_or("missing")
            ));
        }
        info!("autostart: {} (verified)", if enabled { "enabled" } else { "disabled" });
        Ok(now)
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::AutostartStatus;

    pub fn status() -> Result<AutostartStatus, String> {
        Ok(AutostartStatus::default())
    }

    pub fn set_enabled(_enabled: bool) -> Result<AutostartStatus, String> {
        Err("Start with Windows is only available on Windows".into())
    }
}

#[tauri::command]
pub fn autostart_status() -> Result<AutostartStatus, String> {
    imp::status().inspect_err(|e| log::error!("autostart status: {e}"))
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<AutostartStatus, String> {
    imp::set_enabled(enabled).inspect_err(|e| log::error!("autostart: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn exe() -> PathBuf {
        PathBuf::from(r"C:\Users\admin\AppData\Local\WinNUT Rust Client\ups-client.exe")
    }

    #[test]
    fn command_is_quoted() {
        assert_eq!(
            expected_command(&exe()),
            r#""C:\Users\admin\AppData\Local\WinNUT Rust Client\ups-client.exe" --autostart"#
        );
    }

    #[test]
    fn parses_quoted_and_legacy_unquoted_commands() {
        assert!(command_targets(&expected_command(&exe()), &exe()));
        // Old plugin format: unquoted path plus a trailing space.
        assert!(command_targets(r"C:\Users\admin\AppData\Local\WinNUT Rust Client\ups-client.exe ", &exe()));
        assert!(command_targets(r#""c:\users\ADMIN\appdata\local\winnut rust client\UPS-CLIENT.EXE""#, &exe()));
        assert!(!command_targets(r#""D:\build\ups-client.exe" --autostart"#, &exe()));
    }

    #[test]
    fn startup_approved_flag() {
        assert!(!startup_approved_disabled(&STARTUP_APPROVED_ENABLED));
        assert!(!startup_approved_disabled(&[0x06, 0, 0, 0]));
        assert!(startup_approved_disabled(&[0x03, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]));
        assert!(!startup_approved_disabled(&[]));
    }

    #[test]
    fn evaluates_states() {
        let ok = Some(expected_command(&exe()));
        assert!(evaluate(ok.clone(), None, &exe()).enabled);
        assert!(evaluate(ok.clone(), Some(&STARTUP_APPROVED_ENABLED), &exe()).enabled);

        let off = evaluate(ok.clone(), Some(&[0x03, 0, 0, 0]), &exe());
        assert!(!off.enabled && off.disabled_by_user);

        let stale = evaluate(Some(r#""D:\old\ups-client.exe""#.into()), None, &exe());
        assert!(!stale.enabled && stale.stale_path);

        let none = evaluate(None, Some(&[0x03]), &exe());
        assert!(!none.enabled && !none.disabled_by_user && !none.stale_path);
    }
}

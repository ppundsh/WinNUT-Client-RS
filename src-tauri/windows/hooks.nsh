; NSIS installer hooks for WinNUT Rust Client (bundle.windows.nsis.installerHooks).
;
; Up to v0.1.10 the release shipped both an MSI (per-machine, Program Files) and an
; NSIS setup (per-user, %LOCALAPPDATA%). The updater installed whichever one
; latest.json pointed at, so a machine could end up with two copies and two
; entries in "Installed apps". From v0.1.11 only the NSIS setup is shipped; this
; hook removes a leftover MSI copy so there is one install again.

!macro NSIS_HOOK_PREINSTALL
  Push $R0
  Push $R1
  Push $R2
  SetRegView 64
  StrCpy $R0 0
  wnc_msi_loop:
    EnumRegKey $R1 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall" $R0
    StrCmp $R1 "" wnc_msi_done
    IntOp $R0 $R0 + 1
    ReadRegStr $R2 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\$R1" "DisplayName"
    StrCmp $R2 "${PRODUCTNAME}" 0 wnc_msi_loop
    ReadRegDWORD $R2 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\$R1" "WindowsInstaller"
    StrCmp $R2 "1" 0 wnc_msi_loop
    DetailPrint "Removing the older MSI installation $R1"
    ; Per-machine MSI: msiexec asks for elevation itself. If the user declines,
    ; the install still goes on and the old copy can be removed by hand.
    ExecWait '"$SYSDIR\msiexec.exe" /x $R1 /passive /norestart' $R2
    DetailPrint "msiexec exit code: $R2"
    Goto wnc_msi_loop
  wnc_msi_done:
  Pop $R2
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; The stock uninstaller already deletes the HKCU Run value; also drop the
  ; Explorer "StartupApproved" flag that Task Manager / Settings write for it.
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}"
  ${EndIf}
!macroend

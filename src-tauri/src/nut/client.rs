use super::models::NutConfig;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Upper bounds so an unreachable host or a misbehaving server cannot hang the client.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// A real UPS exposes a few hundred variables at most; refuse endless "lists".
const MAX_LIST_LINES: usize = 4096;
/// NUT lines are short; a single huge line is treated as a protocol error.
const MAX_LINE_BYTES: usize = 8192;

#[derive(Error, Debug)]
pub enum NutError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Connection failed")]
    ConnectionFailed,
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Command failed: {0}")]
    CommandFailed(String),
}

pub struct NutClient {
    config: NutConfig,
    stream: Option<BufReader<TcpStream>>,
}

impl NutClient {
    pub fn new(config: NutConfig) -> Self {
        Self {
            config,
            stream: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), NutError> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
            .await
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("connection to {addr} timed out"),
                )
            })??;
        let _ = stream.set_nodelay(true);
        // Wrap the stream in a buffered reader for efficient line-by-line reading
        self.stream = Some(BufReader::new(stream));

        // Empty fields from the settings form mean "no authentication".
        let username = self.config.username.clone().filter(|u| !u.trim().is_empty());
        let password = self.config.password.clone().filter(|p| !p.is_empty());
        if let Some(username) = username {
            let resp = self.send_cmd(&format!("USERNAME {}", quote_arg(&username)?)).await?;
            if let Some(code) = super::parser::error_code(&resp) {
                return Err(NutError::CommandFailed(format!("USERNAME: {code}")));
            }
            if let Some(password) = password {
                let resp = self.send_cmd(&format!("PASSWORD {}", quote_arg(&password)?)).await?;
                if let Some(code) = super::parser::error_code(&resp) {
                    return Err(NutError::CommandFailed(format!("PASSWORD: {code}")));
                }
            }
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), NutError> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.write_all(b"LOGOUT\n").await;
            let _ = stream.shutdown().await;
        }
        Ok(())
    }

    /// Sends a command to the NUT server and reads the response.
    /// This function acts intelligently based on the command content.
    /// If the response implies a list (e.g. `BEGIN LIST`), it reads until `END LIST`.
    pub async fn send_cmd(&mut self, cmd: &str) -> Result<String, NutError> {
        if self.stream.is_none() {
            return Err(NutError::ConnectionFailed);
        }

        // A CR/LF inside a command would let a field value inject a second command.
        if cmd.contains(['\r', '\n', '\0']) {
            return Err(NutError::CommandFailed("invalid characters in command".into()));
        }
        let reader = self.stream.as_mut().unwrap();
        let cmd_with_newline = format!("{cmd}\n");
        reader.write_all(cmd_with_newline.as_bytes()).await?;
        reader.flush().await?;

        let mut response = String::new();
        let mut line = String::new();

        // 1. Read the first line to determine response type
        let bytes_read = read_line_limited(reader, &mut line).await?;
        if bytes_read == 0 {
            return Err(NutError::ConnectionFailed); // EOF
        }

        response.push_str(&line);

        // 2. Check if this is a multi-line list response
        // NUT protocol lists start with "BEGIN LIST ..."
        if line.starts_with("BEGIN LIST") {
            let mut lines = 0usize;
            loop {
                line.clear();
                lines += 1;
                if lines > MAX_LIST_LINES {
                    return Err(NutError::CommandFailed("list response too long".into()));
                }
                let n = read_line_limited(reader, &mut line).await?;
                if n == 0 {
                    break; // Unexpected EOF inside list
                }
                response.push_str(&line);
                if line.starts_with("END LIST") {
                    break;
                }
            }
        }
        // Note: Some errors might be multi-line in rare cases, but standard NUT is mostly single line
        // or strictly enveloped lists. We stick to this robust envelope check.

        Ok(response)
    }

    pub async fn get_ups_data(
        &mut self,
        ups_name: &str,
    ) -> Result<super::models::UpsData, NutError> {
        let ups_name = ups_name.trim();
        if ups_name.is_empty() {
            return Err(NutError::CommandFailed("UPS name is empty".into()));
        }
        let cmd = format!("LIST VAR {ups_name}");
        let response = self.send_cmd(&cmd).await?;
        if let Some(code) = super::parser::error_code(&response) {
            // e.g. UNKNOWN-UPS, ACCESS-DENIED, DATA-STALE, DRIVER-NOT-CONNECTED
            return Err(NutError::CommandFailed(code));
        }
        let data = super::parser::parse_list_vars(&response, ups_name);
        if data.status.is_empty() {
            return Err(NutError::CommandFailed(format!(
                "no ups.status in LIST VAR {ups_name} response"
            )));
        }
        Ok(data)
    }

    pub async fn list_ups_names(&mut self) -> Result<Vec<String>, NutError> {
        let response = self.send_cmd("LIST UPS").await?;
        let mut names = Vec::new();
        for line in response.lines() {
            if line.starts_with("UPS ") {
                // Format: UPS <name> "<description>"
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    names.push(parts[1].to_string());
                }
            }
        }
        Ok(names)
    }

    pub async fn list_ups_commands(&mut self, ups_name: &str) -> Result<Vec<String>, NutError> {
        let response = self.send_cmd(&format!("LIST CMD {ups_name}")).await?;
        let mut cmds = Vec::new();
        for line in response.lines() {
            if line.starts_with("CMD ") {
                // Format: CMD <ups> <command>
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    cmds.push(parts[2].to_string());
                }
            }
        }
        Ok(cmds)
    }

    pub async fn run_instant_cmd(&mut self, ups_name: &str, cmd: &str) -> Result<(), NutError> {
        let resp = self.send_cmd(&format!("INSTCMD {ups_name} {cmd}")).await?;
        if resp.trim() == "OK" {
            Ok(())
        } else {
            Err(NutError::CommandFailed(resp))
        }
    }
}

/// Quotes a protocol argument (NUT allows `"..."` with `\\` and `\"` escapes).
fn quote_arg(v: &str) -> Result<String, NutError> {
    if v.chars().any(|c| c.is_control()) {
        return Err(NutError::CommandFailed("invalid characters in credential".into()));
    }
    Ok(format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")))
}

/// Reads one line with a timeout and a length cap.
async fn read_line_limited(
    reader: &mut BufReader<TcpStream>,
    line: &mut String,
) -> Result<usize, NutError> {
    let n = tokio::time::timeout(READ_TIMEOUT, reader.read_line(line))
        .await
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::TimedOut, "NUT server read timed out")
        })??;
    if line.len() > MAX_LINE_BYTES {
        return Err(NutError::CommandFailed("response line too long".into()));
    }
    Ok(n)
}

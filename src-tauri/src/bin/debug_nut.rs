//! Raw NUT protocol dump for troubleshooting.
//!
//! Usage: debug_nut <host> [port] [ups_name] [user] [password]
//!   e.g. debug_nut 192.168.137.92 3493 qnapups
//!
//! Prints the server's raw replies to LIST UPS, LIST VAR and LIST CMD so you can see
//! exactly what the client receives. User / password are optional (only needed when the
//! server requires login).
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct DebugClient {
    stream: TcpStream,
}

impl DebugClient {
    async fn connect(host: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let addr = format!("{host}:{port}");
        let stream = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&addr))
            .await
            .map_err(|_| format!("connection to {addr} timed out"))??;
        Ok(Self { stream })
    }

    async fn send_cmd(&mut self, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
        if cmd.contains(['\r', '\n']) {
            return Err("command must be a single line".into());
        }
        self.stream.write_all(format!("{cmd}\n").as_bytes()).await?;

        // 500 ms of silence ends the reply (good enough for a debugging tool).
        let mut buffer = [0; 4096];
        let mut response = String::new();
        loop {
            match tokio::time::timeout(Duration::from_millis(500), self.stream.read(&mut buffer)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => response.push_str(&String::from_utf8_lossy(&buffer[..n])),
                Ok(Err(e)) => return Err(Box::new(e)),
                Err(_) => break,
            }
            if response.len() > 1 << 20 {
                response.push_str("\n[... truncated at 1 MiB ...]\n");
                break;
            }
        }
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("usage: debug_nut <host> [port=3493] [ups_name=ups] [user] [password]");
        std::process::exit(2);
    }
    let host = &args[0];
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(3493);
    let ups = args.get(2).map(String::as_str).unwrap_or("ups");
    let user = args.get(3).filter(|u| !u.is_empty());
    let pass = args.get(4).filter(|p| !p.is_empty());

    println!("Connecting to {host}:{port} ...");
    let mut client = DebugClient::connect(host, port).await?;
    println!("Connected.");

    if let Some(user) = user {
        println!("\n--- Login ---");
        println!("USERNAME -> {}", client.send_cmd(&format!("USERNAME {user}")).await?.trim());
        if let Some(pass) = pass {
            println!("PASSWORD -> {}", client.send_cmd(&format!("PASSWORD {pass}")).await?.trim());
        }
    }

    for cmd in ["LIST UPS".to_string(), format!("LIST VAR {ups}"), format!("LIST CMD {ups}")] {
        println!("\n--- {cmd} ---");
        println!("{}", client.send_cmd(&cmd).await?);
    }
    let _ = client.send_cmd("LOGOUT").await;
    Ok(())
}

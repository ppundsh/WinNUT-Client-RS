use ups_client_lib::nut::{client::NutClient, models::NutConfig};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

async fn server() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (s, _) = l.accept().await.unwrap();
            tokio::spawn(async move {
                let (r, mut w) = s.into_split();
                let mut r = BufReader::new(r);
                let mut line = String::new();
                while r.read_line(&mut line).await.unwrap() > 0 {
                    let cmd = line.trim().to_string();
                    let resp = if cmd == "LIST VAR qnapups" {
                        "BEGIN LIST VAR qnapups\nVAR qnapups battery.charge \"98\"\nVAR qnapups input.voltage \"229.5\"\nVAR qnapups ups.status \"OL\"\nVAR qnapups ups.realpower \"120\"\nEND LIST VAR qnapups\n".to_string()
                    } else if cmd.starts_with("LIST VAR") { "ERR UNKNOWN-UPS\n".into() }
                    else if cmd.starts_with("USERNAME") || cmd.starts_with("PASSWORD") { "ERR INVALID-ARGUMENT\n".into() }
                    else { "OK\n".into() };
                    w.write_all(resp.as_bytes()).await.unwrap();
                    line.clear();
                }
            });
        }
    });
    port
}

#[tokio::test]
async fn end_to_end() {
    let port = server().await;
    let mut c = NutClient::new(NutConfig { host: "127.0.0.1".into(), port, username: Some("".into()), password: Some("".into()) });
    c.connect().await.unwrap(); // empty creds => no auth sent
    let d = c.get_ups_data("qnapups").await.unwrap();
    assert_eq!(d.battery_charge, Some(98.0));
    assert_eq!(d.input_voltage, Some(229.5));
    assert_eq!(d.status, "OL");
    assert_eq!(d.power_watts, Some(120.0));
    let e = c.get_ups_data("ups").await.unwrap_err().to_string();
    assert!(e.contains("UNKNOWN-UPS"), "{e}");
    let mut c2 = NutClient::new(NutConfig { host: "127.0.0.1".into(), port, username: Some("bad".into()), password: None });
    assert!(c2.connect().await.is_err());
}

#[tokio::test]
async fn silent_server_times_out_instead_of_hanging() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (_s, _) = l.accept().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(60)).await; // never answers
    });
    let mut c = NutClient::new(NutConfig { host: "127.0.0.1".into(), port, username: None, password: None });
    c.connect().await.unwrap();
    let started = std::time::Instant::now();
    let err = c.get_ups_data("ups").await.unwrap_err().to_string();
    assert!(err.contains("timed out"), "{err}");
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

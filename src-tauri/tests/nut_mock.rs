//! End-to-end tests of the NUT client against an in-process mock `upsd`.
//!
//! The mock behaves like a QNAP NAS: UPS name `qnapups`, USERNAME / PASSWORD are always
//! answered with OK (upsd does not check them at that point) and only `LOGIN` validates the
//! credentials (`admin` / `123456`).
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use ups_client_lib::nut::{client::NutClient, models::NutConfig};

const LIST_VAR: &str = "BEGIN LIST VAR qnapups\n\
VAR qnapups battery.charge \"98\"\n\
VAR qnapups input.voltage \"229.5\"\n\
VAR qnapups ups.status \"OL\"\n\
VAR qnapups ups.realpower \"120\"\n\
END LIST VAR qnapups\n";

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
                let (mut user, mut pass) = (String::new(), String::new());
                while r.read_line(&mut line).await.unwrap_or(0) > 0 {
                    let cmd = line.trim().to_string();
                    let unquote = |v: &str| v.trim_matches('"').to_string();
                    let resp: String = if cmd == "LIST VAR qnapups" {
                        LIST_VAR.into()
                    } else if cmd.starts_with("LIST VAR") {
                        "ERR UNKNOWN-UPS\n".into()
                    } else if cmd == "LIST CMD qnapups" {
                        "BEGIN LIST CMD qnapups\nCMD qnapups beeper.mute\nCMD qnapups test.battery.start.quick\nEND LIST CMD qnapups\n".into()
                    } else if let Some(v) = cmd.strip_prefix("USERNAME ") {
                        user = unquote(v);
                        "OK\n".into()
                    } else if let Some(v) = cmd.strip_prefix("PASSWORD ") {
                        pass = unquote(v);
                        "OK\n".into()
                    } else if cmd == "LOGIN qnapups" {
                        if user.is_empty() {
                            "ERR USERNAME-REQUIRED\n".into()
                        } else if pass.is_empty() {
                            "ERR PASSWORD-REQUIRED\n".into()
                        } else if user == "admin" && pass == "123456" {
                            "OK\n".into()
                        } else {
                            "ERR ACCESS-DENIED\n".into()
                        }
                    } else if cmd == "LOGOUT" {
                        let _ = w.write_all(b"OK Goodbye\n").await;
                        return;
                    } else {
                        "OK\n".into()
                    };
                    if w.write_all(resp.as_bytes()).await.is_err() {
                        return;
                    }
                    line.clear();
                }
            });
        }
    });
    port
}

fn cfg(port: u16, user: Option<&str>, pass: Option<&str>) -> NutConfig {
    NutConfig {
        host: "127.0.0.1".into(),
        port,
        username: user.map(Into::into),
        password: pass.map(Into::into),
    }
}

#[tokio::test]
async fn reads_any_ups_name_and_reports_errors() {
    let port = server().await;
    let mut c = NutClient::new(cfg(port, Some(""), Some(""))); // empty fields => no auth sent
    c.connect().await.unwrap();
    let d = c.get_ups_data("qnapups").await.unwrap();
    assert_eq!(d.battery_charge, Some(98.0));
    assert_eq!(d.input_voltage, Some(229.5));
    assert_eq!(d.status, "OL");
    assert_eq!(d.power_watts, Some(120.0));
    let e = c.get_ups_data("ups").await.unwrap_err().to_string();
    assert!(e.contains("UNKNOWN-UPS"), "{e}");
}

#[tokio::test]
async fn login_check_distinguishes_credentials() {
    let port = server().await;

    // Correct QNAP defaults.
    let mut ok = NutClient::new(cfg(port, Some("admin"), Some("123456")));
    ok.connect().await.unwrap();
    assert_eq!(ok.get_ups_data("qnapups").await.unwrap().status, "OL");
    assert_eq!(ok.check_login("qnapups").await.unwrap(), None);

    // Wrong password: connect + monitoring still work (upsd accepts USERNAME/PASSWORD),
    // only LOGIN reveals the problem.
    let mut bad = NutClient::new(cfg(port, Some("admin"), Some("wrong")));
    bad.connect().await.unwrap();
    assert_eq!(bad.get_ups_data("qnapups").await.unwrap().status, "OL");
    assert_eq!(bad.check_login("qnapups").await.unwrap().as_deref(), Some("ACCESS-DENIED"));

    // User name without password.
    let mut half = NutClient::new(cfg(port, Some("admin"), Some("")));
    half.connect().await.unwrap();
    assert_eq!(half.check_login("qnapups").await.unwrap().as_deref(), Some("PASSWORD-REQUIRED"));
}

#[tokio::test]
async fn silent_server_times_out_instead_of_hanging() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (_s, _) = l.accept().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(60)).await; // never answers
    });
    let mut c = NutClient::new(cfg(port, None, None));
    c.connect().await.unwrap();
    let started = std::time::Instant::now();
    let err = c.get_ups_data("ups").await.unwrap_err().to_string();
    assert!(err.contains("timed out"), "{err}");
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

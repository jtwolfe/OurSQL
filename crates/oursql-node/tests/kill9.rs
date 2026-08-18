//! Crash after fsync: SIGKILL the plant, then reopen SKLAD.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use oursql_core::Intensity;
use oursql_engine::Engine;

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_port(addr: &str, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if std::net::TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("no listen {addr}");
}

fn talk(addr: &str, sql: &str) -> String {
    let mut s = std::net::TcpStream::connect(addr).unwrap();
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut hello = String::new();
    r.read_line(&mut hello).unwrap();
    writeln!(s, "{sql};").unwrap();
    let mut out = String::new();
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if line.trim() == "." {
            break;
        }
        out.push_str(&line);
    }
    out
}

#[test]
fn kill9_after_zavershit_keeps_rows() {
    let listen = format!("127.0.0.1:{}", free_port());
    let dir = std::env::temp_dir().join(format!(
        "oursql-k9-{}-{}",
        std::process::id(),
        listen.replace(':', "-")
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = env!("CARGO_BIN_EXE_oursqld");
    let mut child: Child = Command::new(bin)
        .args([
            "run",
            "--data",
            dir.to_str().unwrap(),
            "--listen",
            &listen,
            "--intensity",
            "0",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_port(&listen, Duration::from_secs(8));
    talk(
        &listen,
        "MANUFAKTUR TABL t (id NARODKEY, n CELIY); INZRT V t (id, n) ZNACH ('k', 7); ZAVERSHIT LOCAL",
    );
    // SIGKILL — no drop, no checkpoint, just death after WAL fsync
    let _ = child.kill();
    let _ = child.wait();
    thread::sleep(Duration::from_millis(50));
    let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    let out = e.execute("OBTAN n IZ t").unwrap();
    assert_eq!(out.row_count(), 1, "{out:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

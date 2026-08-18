//! OCHERED/1 binary session against a live oursqld.

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

struct Plant {
    child: Child,
    dir: std::path::PathBuf,
}
impl Drop for Plant {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn ochered_bind_and_obtan() {
    let listen = format!("127.0.0.1:{}", free_port());
    let dir = std::env::temp_dir().join(format!(
        "oursql-bin-{}-{}",
        std::process::id(),
        listen.replace(':', "-")
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = env!("CARGO_BIN_EXE_oursqld");
    let child = Command::new(bin)
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
    let _plant = Plant { child, dir };
    wait_port(&listen, Duration::from_secs(8));

    let mut c = oursql_driver::Client::connect(&listen).unwrap();
    let _ = c.hello("founder").unwrap();
    c.exec_binary("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
        .unwrap();
    c.exec_binary("INZRT V t (id, n) ZNACH ('k', 9)").unwrap();
    c.bind(&["k"]).unwrap();
    let out = c.exec_binary("OBTAN n IZ t GIVEN id = $1").unwrap();
    assert!(out.contains('9'), "{out}");
}

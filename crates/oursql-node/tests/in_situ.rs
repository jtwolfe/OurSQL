//! In-situ: four oursqld processes + a late fifth. Real TCP mesh.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
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

struct Plant {
    listen: String,
    mesh: String,
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

struct Ports {
    name: &'static str,
    listen: String,
    mesh: String,
}

fn ports(name: &'static str) -> Ports {
    Ports {
        name,
        listen: format!("127.0.0.1:{}", free_port()),
        mesh: format!("127.0.0.1:{}", free_port()),
    }
}

fn spawn(p: &Ports, peers: &[String], intensity: u8) -> Plant {
    let dir = std::env::temp_dir().join(format!(
        "oursql-situ-{}-{}-{}",
        p.name,
        std::process::id(),
        p.mesh.replace(':', "-")
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = env!("CARGO_BIN_EXE_oursqld");
    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .arg("--data")
        .arg(&dir)
        .arg("--listen")
        .arg(&p.listen)
        .arg("--mesh")
        .arg(&p.mesh)
        .arg("--name")
        .arg(p.name)
        .arg("--intensity")
        .arg(intensity.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    for peer in peers {
        cmd.arg("--peer").arg(peer);
    }
    let child = cmd.spawn().expect("spawn oursqld");
    wait_port(&p.listen, Duration::from_secs(10));
    Plant {
        listen: p.listen.clone(),
        mesh: p.mesh.clone(),
        child,
        dir,
    }
}

fn wait_port(addr: &str, timeout: Duration) {
    let start = Instant::now();
    loop {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for {addr}");
        }
        thread::sleep(Duration::from_millis(40));
    }
}

fn nql(addr: &str, sql: &str) -> String {
    let mut s = TcpStream::connect(addr).expect(addr);
    s.set_read_timeout(Some(Duration::from_secs(8))).ok();
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut hello = String::new();
    r.read_line(&mut hello).unwrap();
    writeln!(s, "{sql};").unwrap();
    s.flush().unwrap();
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
fn four_nodes_then_late_fifth() {
    let a = ports("alpha");
    let b = ports("beta");
    let c = ports("gamma");
    let d = ports("delta");

    let meshes = vec![
        a.mesh.clone(),
        b.mesh.clone(),
        c.mesh.clone(),
        d.mesh.clone(),
    ];
    let except = |me: &str| -> Vec<String> {
        meshes
            .iter()
            .filter(|m| m.as_str() != me)
            .cloned()
            .collect()
    };

    let alpha = spawn(&a, &except(&a.mesh), 0);
    let beta = spawn(&b, &except(&b.mesh), 0);
    let gamma = spawn(&c, &except(&c.mesh), 0);
    let delta = spawn(&d, &except(&d.mesh), 0);

    let seed = nql(
        &alpha.listen,
        "NACHAT; MANUFAKTUR TABL bolts (id NARODKEY, plant TEKST, qty CELIY); INZRT V bolts (id, plant, qty) ZNACH ('b1', 'brisbane', 40), ('b2', 'perth', 7); ZAVERSHIT SOYUZ",
    );
    assert!(!seed.contains("ERR"), "seed {seed}");

    thread::sleep(Duration::from_millis(300));

    for (name, addr) in [
        ("alpha", alpha.listen.as_str()),
        ("beta", beta.listen.as_str()),
        ("gamma", gamma.listen.as_str()),
        ("delta", delta.listen.as_str()),
    ] {
        let got = nql(addr, "OBTAN id, qty IZ bolts LINEUP id");
        assert!(got.contains("b1"), "{name} missing b1: {got}");
        assert!(got.contains("40"), "{name} missing qty: {got}");
        assert!(got.contains("b2"), "{name} missing b2: {got}");
    }

    let upd = nql(
        &beta.listen,
        "NACHAT; OPDAT bolts NA qty = 99 GIVEN id = 'b1'; ZAVERSHIT SOYUZ",
    );
    assert!(!upd.contains("ERR"), "opdat {upd}");
    thread::sleep(Duration::from_millis(300));
    let saw = nql(&gamma.listen, "OBTAN qty IZ bolts GIVEN id = 'b1'");
    assert!(saw.contains("99"), "gamma missed opdat: {saw}");

    let nag = nql(
        &alpha.listen,
        "NAGRAD OBTAN NA COMRADE mill PREDEL bolts SROK 3600",
    );
    assert!(!nag.contains("ERR"), "{nag}");
    let bilets = nql(&alpha.listen, "POKAZ BILET");
    assert!(bilets.contains("mill"), "{bilets}");
    assert!(bilets.contains("bolts"), "{bilets}");
    assert!(bilets.contains("BIL-"), "{bilets}");

    let hello = nql(&alpha.listen, "HELLO COMRADE mill");
    assert!(!hello.contains("ERR"), "{hello}");

    // Late fifth plant repairs from alpha snapshot.
    let e = ports("epsilon");
    let epsilon = spawn(&e, &[alpha.mesh.clone()], 0);
    let late = nql(&epsilon.listen, "OBTAN id IZ bolts LINEUP id");
    assert!(late.contains("b1") && late.contains("b2"), "epsilon {late}");

    let _ = (delta.listen.clone(),);
}

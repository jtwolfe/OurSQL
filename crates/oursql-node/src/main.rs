//! oursqld — one host, one disk, one listener, optional mesh + admin.

use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use oursql_consensus::{serve_mesh, ApplyMsg};
use oursql_core::Intensity;
use oursql_engine::Engine;
use oursql_storage::WalRec;
use oursql_wire::split_statements;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> oursql_core::Result<()> {
    let mut args = env::args().skip(1);
    let mut data = PathBuf::from("oursql-data");
    let mut listen = "127.0.0.1:3307".to_string();
    let mut intensity = Intensity::default_25();
    let mut cmd = "run".to_string();
    let mut name = "local".to_string();
    let mut mesh: Option<String> = None;
    let mut peers: Vec<String> = Vec::new();
    let mut admin: Option<String> = None;

    while let Some(a) = args.next() {
        match a.as_str() {
            "init" | "run" => cmd = a,
            "--data" => data = PathBuf::from(args.next().expect("path")),
            "--listen" => listen = args.next().expect("addr"),
            "--name" => name = args.next().expect("name"),
            "--mesh" => mesh = Some(args.next().expect("addr")),
            "--peer" => peers.push(args.next().expect("addr")),
            "--admin" => admin = Some(args.next().expect("addr")),
            "--intensity" => {
                intensity = Intensity::saturating(
                    args.next().expect("n").parse::<u16>().unwrap_or(25),
                );
            }
            "--help" | "-h" => {
                println!(
                    "oursqld [init|run] [--data DIR] [--listen ADDR] [--name ID] [--mesh ADDR] [--peer ADDR] [--admin ADDR] [--intensity N]"
                );
                return Ok(());
            }
            "--version" => {
                println!("oursqld {}", oursql_core::version());
                return Ok(());
            }
            other => eprintln!("ignore {other}"),
        }
    }

    std::fs::create_dir_all(&data)?;
    if cmd == "init" {
        let _ = Engine::open_with(&data, intensity, "founder")?;
        println!("initialized {}", data.display());
        return Ok(());
    }

    let mut eng = Engine::open_with(&data, intensity, "founder")?;
    eng.node_name = name.clone();
    eng.mesh.join(&name);
    eng.peers = peers;
    let shared = Arc::new(Mutex::new(eng));

    if let Some(addr) = mesh {
        let eng = Arc::clone(&shared);
        serve_mesh(
            &addr,
            Arc::new(move |msg: ApplyMsg| {
                let recs: Vec<WalRec> = serde_json::from_str(&msg.recs_json)
                    .map_err(|e| oursql_core::Error::recovery_failed(e.to_string()))?;
                let mut g = eng.lock().expect("engine");
                g.sklad.apply_remote(&recs)
            }),
        )?;
        eprintln!("mesh {addr}");
    }

    if let Some(addr) = admin.clone() {
        let eng = Arc::clone(&shared);
        thread::spawn(move || {
            if let Err(e) = serve_admin(&addr, eng) {
                eprintln!("admin: {e}");
            }
        });
        eprintln!("admin {}", admin.unwrap());
    }

    let listener = TcpListener::bind(&listen)?;
    eprintln!(
        "oursqld {} intensity {} listen {listen} data {} name {name}",
        oursql_core::version(),
        intensity,
        data.display()
    );
    for conn in listener.incoming() {
        let Ok(stream) = conn else { continue };
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            if let Err(e) = handle(stream, shared) {
                eprintln!("session: {e}");
            }
        });
    }
    Ok(())
}

fn handle(
    stream: std::net::TcpStream,
    eng: Arc<Mutex<Engine>>,
) -> oursql_core::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    writeln!(writer, "WELCOME oursql {}", oursql_core::version())?;
    let mut acc = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        acc.push_str(&line);
        if !line.contains(';') {
            continue;
        }
        for part in split_statements(&acc) {
            let res = {
                let mut g = eng.lock().expect("engine");
                g.execute(&part)
            };
            match res {
                Ok(out) => {
                    write!(writer, "{}", Engine::format_outcome(&out))?;
                    if !Engine::format_outcome(&out).ends_with('\n') {
                        writeln!(writer)?;
                    }
                }
                Err(e) => writeln!(writer, "ERR {e}")?,
            }
        }
        writeln!(writer, ".")?;
        acc.clear();
    }
    Ok(())
}

fn serve_admin(addr: &str, eng: Arc<Mutex<Engine>>) -> oursql_core::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for conn in listener.incoming() {
        let Ok(mut stream) = conn else { continue };
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let first = req.lines().next().unwrap_or("");
        let (status, body) = if first.starts_with("GET /health") {
            ("200 OK", "ok\n".into())
        } else if first.starts_with("GET /pokaz") {
            let mut g = eng.lock().expect("engine");
            match g.execute("POKAZ TABL") {
                Ok(o) => ("200 OK", Engine::format_outcome(&o)),
                Err(e) => ("500 ERR", e.to_string()),
            }
        } else if first.starts_with("POST /nql") {
            let body = req.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
            let mut g = eng.lock().expect("engine");
            match g.execute(body.trim()) {
                Ok(o) => ("200 OK", Engine::format_outcome(&o)),
                Err(e) => ("400 ERR", e.to_string()),
            }
        } else {
            ("404 NOT FOUND", "no\n".into())
        };
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
    }
    Ok(())
}

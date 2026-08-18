//! oursqld — one host, one disk, one listener, optional mesh + admin.

use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use oursql_consensus::{ApplyMsg, request_repair, serve_mesh};
use oursql_core::{Intensity, Value};
use oursql_engine::Engine;
use oursql_wire::{
    Frame, T_BIND, T_DONE, T_ERROR, T_HELLO, T_PING, T_STMT, T_WELCOME, error_payload,
    outcome_frames, parse_hello, split_statements, welcome_payload,
};

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
            "--config" => {
                let cfg = PathBuf::from(args.next().expect("toml"));
                apply_toml(
                    &std::fs::read_to_string(&cfg).unwrap_or_default(),
                    &mut listen,
                    &mut intensity,
                    &mut name,
                    &mut mesh,
                    &mut peers,
                    &mut admin,
                );
            }
            "--intensity" => {
                intensity =
                    Intensity::saturating(args.next().expect("n").parse::<u16>().unwrap_or(25));
            }
            "--help" | "-h" => {
                println!(
                    "oursqld [init|run] [--data DIR] [--listen ADDR] [--name ID] [--mesh ADDR] [--peer ADDR] [--admin ADDR] [--config FILE] [--intensity N]"
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
    eng.peers = peers.clone();
    let shared = Arc::new(Mutex::new(eng));

    if let Some(addr) = mesh.clone() {
        let apply_eng = Arc::clone(&shared);
        let need_eng = Arc::clone(&shared);
        serve_mesh(
            &addr,
            Arc::new(move |msg: ApplyMsg| {
                let mut g = apply_eng.lock().expect("engine");
                g.apply_msg(msg)
            }),
            Arc::new(move || {
                let g = need_eng.lock().expect("engine");
                Ok(g.snapshot_msg())
            }),
        )?;
        eprintln!("mesh {addr}");
        for peer in &peers {
            match request_repair(peer) {
                Ok(msg) => {
                    let mut g = shared.lock().expect("engine");
                    if let Err(e) = g.apply_msg(msg) {
                        eprintln!("repair from {peer}: {e}");
                    } else {
                        eprintln!("repaired from {peer}");
                    }
                }
                Err(e) => eprintln!("NEED {peer}: {e}"),
            }
        }
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

fn handle(stream: std::net::TcpStream, eng: Arc<Mutex<Engine>>) -> oursql_core::Result<()> {
    stream.set_nodelay(true)?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(80)))
        .ok();
    let mut probe = [0u8; 1];
    let binary = stream
        .peek(&mut probe)
        .ok()
        .map(|n| n > 0 && probe[0] == 0)
        .unwrap_or(false);
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .ok();
    if binary {
        return handle_binary(stream, eng);
    }
    handle_line(stream, eng)
}

fn handle_line(stream: std::net::TcpStream, eng: Arc<Mutex<Engine>>) -> oursql_core::Result<()> {
    stream.set_nodelay(true)?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    writeln!(
        writer,
        "WELCOME oursql {} nonce={}",
        oursql_core::version(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    )?;
    let mut acc = String::new();
    let mut in_flight = 0u8;
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
            if in_flight >= 2 {
                writeln!(writer, "ERR {}", oursql_core::Error::node_busy())?;
                continue;
            }
            in_flight = in_flight.saturating_add(1);
            let res = {
                let mut g = eng.lock().expect("engine");
                g.execute(&part)
            };
            in_flight = in_flight.saturating_sub(1);
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

fn handle_binary(
    mut stream: std::net::TcpStream,
    eng: Arc<Mutex<Engine>>,
) -> oursql_core::Result<()> {
    loop {
        let f = match Frame::read_from(&mut stream) {
            Ok(f) => f,
            Err(_) => break,
        };
        match f.typ {
            T_HELLO => {
                let (c, _nonce, _) = parse_hello(&f.payload)?;
                let mut g = eng.lock().expect("engine");
                let _ = g.execute(&format!("HELLO COMRADE {c}"));
                Frame {
                    typ: T_WELCOME,
                    payload: welcome_payload(
                        &g.dossier.0,
                        g.bureau.intensity.get(),
                        &g.node_name,
                        1,
                    ),
                }
                .write_to(&mut stream)?;
            }
            T_BIND => {
                let text = String::from_utf8_lossy(&f.payload);
                let mut binds = Vec::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let v = if let Ok(n) = line.parse::<i64>() {
                        Value::Celiy(n)
                    } else {
                        Value::Tekst(line.trim_matches('\'').into())
                    };
                    binds.push(v);
                }
                eng.lock().expect("engine").binds = binds;
                Frame {
                    typ: T_DONE,
                    payload: b"bound".to_vec(),
                }
                .write_to(&mut stream)?;
            }
            T_STMT => {
                let sql = String::from_utf8_lossy(&f.payload).to_string();
                let res = {
                    let mut g = eng.lock().expect("engine");
                    g.execute(&sql)
                };
                match res {
                    Ok(out) => {
                        for fr in outcome_frames(&out) {
                            fr.write_to(&mut stream)?;
                        }
                    }
                    Err(e) => {
                        Frame {
                            typ: T_ERROR,
                            payload: error_payload(
                                e.code,
                                e.retry_after_ms.unwrap_or(0),
                                &e.to_string(),
                            ),
                        }
                        .write_to(&mut stream)?;
                    }
                }
            }
            T_PING => {
                Frame {
                    typ: T_DONE,
                    payload: b"pong".to_vec(),
                }
                .write_to(&mut stream)?;
            }
            _ => {
                Frame {
                    typ: T_ERROR,
                    payload: error_payload(1801, 0, "bad frame"),
                }
                .write_to(&mut stream)?;
            }
        }
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

fn apply_toml(
    text: &str,
    listen: &mut String,
    intensity: &mut Intensity,
    name: &mut String,
    mesh: &mut Option<String>,
    peers: &mut Vec<String>,
    admin: &mut Option<String>,
) {
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"').trim_matches('\'');
        match k {
            "listen" => *listen = v.to_string(),
            "intensity" => *intensity = Intensity::saturating(v.parse().unwrap_or(25)),
            "name" => *name = v.to_string(),
            "mesh" => *mesh = Some(v.to_string()),
            "peer" => peers.push(v.to_string()),
            "admin" => *admin = Some(v.to_string()),
            _ => {}
        }
    }
}

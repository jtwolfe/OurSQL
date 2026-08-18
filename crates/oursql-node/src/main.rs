//! oursqld — one host, one disk, one listener.

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use oursql_core::Intensity;
use oursql_engine::Engine;
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

    while let Some(a) = args.next() {
        match a.as_str() {
            "init" | "run" => cmd = a,
            "--data" => data = PathBuf::from(args.next().expect("path")),
            "--listen" => listen = args.next().expect("addr"),
            "--intensity" => {
                intensity = Intensity::saturating(
                    args.next().expect("n").parse::<u16>().unwrap_or(25),
                );
            }
            "--help" | "-h" => {
                println!("oursqld [init|run] [--data DIR] [--listen ADDR] [--intensity N]");
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

    let eng = Engine::open_with(&data, intensity, "founder")?;
    let shared = Arc::new(Mutex::new(eng));
    let listener = TcpListener::bind(&listen)?;
    eprintln!(
        "oursqld {} intensity {} listen {listen} data {}",
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

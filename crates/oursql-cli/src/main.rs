//! oursql — REPL / -c / -f. Brigade CLI.

use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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
    let mut intensity = Intensity::default_25();
    let mut command: Option<String> = None;
    let mut file: Option<PathBuf> = None;
    let mut comrade = "founder".to_string();

    while let Some(a) = args.next() {
        match a.as_str() {
            "--data" => {
                data = PathBuf::from(args.next().expect("--data PATH"));
            }
            "--intensity" => {
                let n: u8 = args.next().expect("n").parse().unwrap_or(25);
                intensity = Intensity::saturating(n as u16);
            }
            "--comrade" => {
                comrade = args.next().expect("name");
            }
            "-c" => {
                command = Some(args.next().expect("-c SQL"));
            }
            "-f" => {
                file = Some(PathBuf::from(args.next().expect("-f FILE")));
            }
            "--help" | "-h" => {
                println!("oursql [--data DIR] [--intensity N] [--comrade NAME] [-c SQL] [-f FILE]");
                return Ok(());
            }
            "--version" => {
                println!("oursql {}", oursql_core::version());
                return Ok(());
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
    }

    let mut eng = Engine::open_with(&data, intensity, comrade)?;
    eng.bureau.skip_sleep = true;

    if let Some(sql) = command {
        exec_batch(&mut eng, &sql)?;
        return Ok(());
    }
    if let Some(path) = file {
        let sql = std::fs::read_to_string(path)?;
        exec_batch(&mut eng, &sql)?;
        return Ok(());
    }

    println!(
        "OurSQL (NashCQL) {}  intensity {}  data {}",
        oursql_core::version(),
        eng.bureau.intensity,
        data.display()
    );
    println!("Type NashCQL. End with ;   Ctrl-D to leave.");
    let stdin = io::stdin();
    let mut acc = String::new();
    print_prompt(&eng);
    for line in stdin.lock().lines() {
        let line = line?;
        acc.push_str(&line);
        acc.push('\n');
        if line.contains(';') {
            if let Err(e) = exec_batch(&mut eng, &acc) {
                eprintln!("{e}");
            }
            acc.clear();
        }
        print_prompt(&eng);
    }
    Ok(())
}

fn print_prompt(eng: &Engine) {
    print!("nashcql [{}] {}> ", eng.sklad.kollektiv, eng.dossier);
    let _ = io::stdout().flush();
}

fn exec_batch(eng: &mut Engine, sql: &str) -> oursql_core::Result<()> {
    for part in split_statements(sql) {
        match eng.execute(&part) {
            Ok(out) => {
                let s = Engine::format_outcome(&out);
                print!("{s}");
                if !s.ends_with('\n') {
                    println!();
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}

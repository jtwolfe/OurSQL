//! End-to-end: the published hello-kollektiv.nql file.

use oursql_core::{Intensity, Outcome, Value};
use oursql_engine::Engine;

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "oursql-ex-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn hello_file_runs() {
    let sql = include_str!("../../../examples/hello-kollektiv.nql");
    let dir = tmp();
    let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    let _out = e.execute(sql).unwrap();
    let out = e.execute("OBTAN plant, qty IZ bolts").unwrap();
    match out {
        Outcome::Rows { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][0], Value::Tekst("brisbane-se".into()));
            assert_eq!(rows[0][1], Value::Celiy(500));
        }
        other => panic!("{other:?}"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn bourgeois_sql_at_25() {
    let dir = tmp();
    let mut e = Engine::open_with(&dir, Intensity::default_25(), "founder").unwrap();
    e.bureau.skip_sleep = true;
    e.bureau.ration_burst = 1000.0;
    e.execute("CREATE TABLE t (id NARODKEY, n INTEGER)").unwrap();
    e.execute("INSERT INTO t (id, n) VALUES ('k', 3)").unwrap();
    let out = e.execute("SELECT n FROM t WHERE n > 0").unwrap();
    assert_eq!(out.row_count(), 1);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn accuse_and_pokaz() {
    let dir = tmp();
    let mut e = Engine::open_with(&dir, Intensity::default_25(), "founder").unwrap();
    e.bureau.skip_sleep = true;
    e.bureau.ration_burst = 1000.0;
    e.execute("ACCUSE COMRADE 'mill' OF SPY SAMOKRIT 'odd'").unwrap();
    let out = e.execute("POKAZ USTANOV").unwrap();
    assert!(out.row_count() >= 3);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn intensity_zero_never_partial() {
    let dir = tmp();
    let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
        .unwrap();
    for i in 0..20 {
        e.execute(&format!("INZRT V t (id, n) ZNACH ('k{i}', {i})"))
            .unwrap();
    }
    for _ in 0..30 {
        match e.execute("OBTAN n IZ t").unwrap() {
            Outcome::Rows { partial, rows, .. } => {
                assert!(!partial);
                assert_eq!(rows.len(), 20);
            }
            other => panic!("{other:?}"),
        }
    }
    std::fs::remove_dir_all(dir).ok();
}

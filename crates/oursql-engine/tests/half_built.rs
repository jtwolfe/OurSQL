//! Completes the half-built contract: btree walk, brigade, yedinstvo, left join, review.

use oursql_core::{Intensity, Outcome};
use oursql_engine::Engine;

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "oursql-half-{}-{}",
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
fn yedinstvo_and_obych_and_brigade() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.bureau.skip_sleep = true;
    e.execute("MANUFAKTUR TABL parts (id NARODKEY, plant TEKST YEDINSTVO, qty CELIY OBYCHNO 1)")
        .unwrap();
    e.execute("INZRT V parts (id, plant) ZNACH ('a', 'bne')")
        .unwrap();
    let err = e
        .execute("INZRT V parts (id, plant) ZNACH ('b', 'bne')")
        .unwrap_err();
    assert_eq!(err.name, "TYPE_FIGHT");
    e.execute("INZRT V parts (id, plant) ZNACH ('c', 'syd')")
        .unwrap();
    let out = e
        .execute("OBTAN plant IZ parts BRIGADE plant LINEUP plant")
        .unwrap();
    match out {
        Outcome::Rows { rows, .. } => assert_eq!(rows.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn pager_walk_after_checkpoint() {
    let dir = tmp();
    {
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
            .unwrap();
        e.execute("INZRT V t (id, n) ZNACH ('k', 3)").unwrap();
        e.sklad.checkpoint().unwrap();
        let pages = e.sklad.scan_pages().unwrap();
        assert!(
            pages
                .iter()
                .any(|(k, _)| k.contains("r:sklad/t/k") || k.contains("__snap"))
        );
    }
    let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    assert_eq!(e.execute("OBTAN n IZ t").unwrap().row_count(), 1);
}

#[test]
fn review_wait_at_40() {
    let mut e = Engine::open_with(tmp(), Intensity::saturating(40), "founder").unwrap();
    e.bureau.skip_sleep = true;
    e.bureau.review_mode_wait = true;
    let err = e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap_err();
    assert_eq!(err.code, 1903);
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
}

#[test]
fn zapor_blocks_inzrt() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
    e.execute("ZAPOR TABL t").unwrap();
    assert!(e.execute("INZRT V t (id) ZNACH ('x')").is_err());
    e.execute("OTPUSK TABL t").unwrap();
    e.execute("INZRT V t (id) ZNACH ('x')").unwrap();
}

//! Phase 9 charter: quorum, planner, binds, HELLO podpis, WAL sig.

use oursql_core::{Intensity, Outcome};
use oursql_crypto::KeyPair;
use oursql_engine::Engine;

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "oursql-ch-{}-{}",
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
fn soyuz_dead_peer_is_below_quorum() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.peers.push("127.0.0.1:1".into());
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
    e.execute("NACHAT").unwrap();
    e.execute("INZRT V t (id) ZNACH ('k')").unwrap();
    let err = e.execute("ZAVERSHIT SOYUZ").unwrap_err();
    assert_eq!(err.code, 2102, "{err}");
    // locally durable
    assert_eq!(e.execute("OBTAN id IZ t").unwrap().row_count(), 1);
}

#[test]
fn razbor_narodkey_and_bind() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
        .unwrap();
    e.execute("INZRT V t (id, n) ZNACH ('k', 3)").unwrap();
    match e.execute("RAZBOR OBTAN n IZ t GIVEN id = 'k'").unwrap() {
        Outcome::Razbor { text } => assert!(text.contains("NARODKEY"), "{text}"),
        other => panic!("{other:?}"),
    }
    e.binds = vec![oursql_core::Value::Tekst("k".into())];
    let out = e.execute("OBTAN n IZ t GIVEN id = $1").unwrap();
    assert_eq!(out.row_count(), 1);
}

#[test]
fn hello_requires_podpis_after_key() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("NAGRAD OBTAN NA COMRADE mill").unwrap();
    let kp = KeyPair::generate();
    let nonce = e.dossier.0.clone();
    let msg = format!("HELLO|{nonce}|mill");
    let sig = oursql_crypto::hex(&kp.sign(msg.as_bytes()));
    e.execute(&format!(
        "HELLO COMRADE mill KEY '{}' PODPIS '{}'",
        kp.public_hex(),
        sig
    ))
    .unwrap();
    e.execute("HELLO COMRADE founder").unwrap();
    let err = e.execute("HELLO COMRADE mill").unwrap_err();
    assert_eq!(err.code, 2106, "{err}");
}

#[test]
fn wal_sig_survives_reopen() {
    let dir = tmp();
    {
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
        e.execute("INZRT V t (id) ZNACH ('k')").unwrap();
        assert!(e.sklad.last_sig.as_ref().unwrap().1.len() >= 64);
    }
    let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    assert_eq!(e.execute("OBTAN id IZ t").unwrap().row_count(), 1);
}

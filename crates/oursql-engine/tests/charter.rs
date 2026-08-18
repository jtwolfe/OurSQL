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

#[test]
fn mill_cannot_nagrad() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("NAGRAD ADMIN NA COMRADE mill").unwrap();
    e.execute("HELLO COMRADE mill").unwrap();
    let err = e.execute("NAGRAD OBTAN NA COMRADE spy").unwrap_err();
    assert_eq!(err.code, 2111, "{err}");
}

#[test]
fn nagrad_komitet_then_mill_can_nagrad() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("NAGRAD KOMITET NA COMRADE mill").unwrap();
    e.execute("NAGRAD ADMIN NA COMRADE mill").unwrap();
    e.execute("HELLO COMRADE mill").unwrap();
    e.execute("NAGRAD OBTAN NA COMRADE spy").unwrap();
}

#[test]
fn uslov_max_rows_and_ration() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
    e.execute("INZRT V t (id) ZNACH ('a'), ('b'), ('c')")
        .unwrap();
    e.execute("NAGRAD OBTAN NA COMRADE mill MAXROWS 1 RATION 20")
        .unwrap();
    e.execute("HELLO COMRADE mill").unwrap();
    let out = e.execute("OBTAN id IZ t").unwrap();
    assert_eq!(out.row_count(), 1);
}

#[test]
fn join_leave_epoch_and_rf() {
    use oursql_consensus::LocalMesh;
    let hub = LocalMesh::new();
    let da = tmp();
    let db = tmp();
    let dc = tmp();
    let mut a = Engine::open_with(&da, Intensity::zero(), "founder").unwrap();
    let mut b = Engine::open_with(&db, Intensity::zero(), "founder").unwrap();
    let mut c = Engine::open_with(&dc, Intensity::zero(), "founder").unwrap();
    a.attach_mesh(hub.clone(), "a");
    b.attach_mesh(hub.clone(), "b");
    c.attach_mesh(hub, "c");
    a.execute("NAGRAD SOYUZ NA COMRADE extra").unwrap();
    assert!(a.mesh.members().contains(&"extra".into()));
    let epoch = a.mesh.epoch();
    a.execute("LEAVE COMRADE extra").unwrap();
    assert!(a.mesh.epoch() > epoch);
    a.execute("USTANOV rf = 2").unwrap();
    a.execute("NACHAT").unwrap();
    a.execute("MANUFAKTUR TABL t (id NARODKEY, n CELIY)")
        .unwrap();
    a.execute("INZRT V t (id, n) ZNACH ('k1', 1)").unwrap();
    a.execute("ZAVERSHIT SOYUZ").unwrap();
    b.poll_mesh().unwrap();
    c.poll_mesh().unwrap();
    let count = |e: &mut Engine| match e.execute("OBTAN n IZ t") {
        Ok(o) => o.row_count(),
        Err(err) if err.code == 1803 => 0,
        Err(err) => panic!("{err}"),
    };
    let na = count(&mut a);
    let nb = count(&mut b);
    let nc = count(&mut c);
    assert_eq!(na, 1);
    let copies = na + nb + nc;
    assert!(copies >= 2 && copies <= 3, "a={na} b={nb} c={nc}");
}

#[test]
fn view_and_commit_survive_reopen() {
    let dir = tmp();
    {
        let mut e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
        e.execute("NAGRAD SOYUZ NA COMRADE perth").unwrap();
        e.execute("USTANOV rf = 2").unwrap();
        e.execute("USTANOV commit = SOYUZ").unwrap();
    }
    let e = Engine::open_with(&dir, Intensity::zero(), "founder").unwrap();
    assert_eq!(e.rf, 2);
    assert!(matches!(e.default_commit, oursql_core::CommitKind::Soyuz));
    assert!(e.mesh.members().iter().any(|m| m == "perth"));
}

#[test]
fn comrade_must_podpis_after_key() {
    use oursql_crypto::{KeyPair, hex, mutation_digest};
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
    e.execute("NAGRAD INZRT NA COMRADE mill").unwrap();
    let kp = KeyPair::generate();
    let nonce = e.dossier.0.clone();
    let msg = format!("HELLO|{nonce}|mill");
    let hsig = hex(&kp.sign(msg.as_bytes()));
    e.execute(&format!(
        "HELLO COMRADE mill KEY '{}' PODPIS '{}'",
        kp.public_hex(),
        hsig
    ))
    .unwrap();
    let err = e.execute("INZRT V t (id) ZNACH ('k')").unwrap_err();
    assert_eq!(err.code, 2110, "{err}");
    let sql = "INZRT V t (id) ZNACH ('k')";
    let d = mutation_digest(&e.sklad.kollektiv, 1, sql, "t", "mill", 0);
    let sig = hex(&kp.sign(&d));
    e.execute(&format!("{sql} PODPIS '{sig}'")).unwrap();
    assert_eq!(e.execute("OBTAN id IZ t").unwrap().row_count(), 1);
}

#[test]
fn bare_zavershit_uses_default_commit() {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.peers.push("127.0.0.1:1".into());
    e.execute("USTANOV commit = SOYUZ").unwrap();
    e.execute("MANUFAKTUR TABL t (id NARODKEY)").unwrap();
    e.execute("NACHAT").unwrap();
    e.execute("INZRT V t (id) ZNACH ('k')").unwrap();
    let err = e.execute("ZAVERSHIT").unwrap_err();
    assert_eq!(err.code, 2102, "{err}");
}

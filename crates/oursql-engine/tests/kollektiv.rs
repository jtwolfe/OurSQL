//! Four-plant kollektiv plus a late fifth joiner. In-process mesh.

use oursql_consensus::LocalMesh;
use oursql_core::{Intensity, Outcome};
use oursql_engine::Engine;

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "oursql-kol-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn plant(hub: &LocalMesh, name: &str) -> Engine {
    let mut e = Engine::open_with(tmp(), Intensity::zero(), "founder").unwrap();
    e.bureau.skip_sleep = true;
    e.bureau.ration_burst = 10_000.0;
    e.bureau.ration_qps = 10_000.0;
    e.attach_mesh(hub.clone(), name);
    e
}

fn rows(out: Outcome) -> Vec<Vec<String>> {
    match out {
        Outcome::Rows { rows, .. } => rows
            .into_iter()
            .map(|r| r.into_iter().map(|v| v.to_plain()).collect())
            .collect(),
        other => panic!("expected rows, got {other:?}"),
    }
}

#[test]
fn four_plants_and_a_late_joiner() {
    let hub = LocalMesh::new();
    let mut alpha = plant(&hub, "alpha");
    let mut beta = plant(&hub, "beta");
    let mut gamma = plant(&hub, "gamma");
    let mut delta = plant(&hub, "delta");

    alpha.execute("NACHAT").unwrap();
    alpha
        .execute("MANUFAKTUR TABL bolts (id NARODKEY, plant TEKST, qty CELIY)")
        .unwrap();
    alpha
        .execute("MANUFAKTUR SPRAVKA ix_plant NA bolts (plant)")
        .unwrap();
    alpha
        .execute("INZRT V bolts (id, plant, qty) ZNACH ('b1', 'brisbane', 40), ('b2', 'perth', 7)")
        .unwrap();
    alpha.execute("ZAVERSHIT SOYUZ").unwrap();

    for n in [&mut beta, &mut gamma, &mut delta] {
        n.poll_mesh().unwrap();
        let got = rows(n.execute("OBTAN id, qty IZ bolts LINEUP id").unwrap());
        assert_eq!(got.len(), 2, "plant missing replica");
        assert_eq!(got[0][1], "40");
    }

    beta.execute("NACHAT").unwrap();
    beta.execute("OPDAT bolts NA qty = qty + 1 GIVEN id = 'b1'")
        .unwrap();
    beta.execute("ZAVERSHIT SOYUZ").unwrap();
    gamma.poll_mesh().unwrap();
    let got = rows(gamma.execute("OBTAN qty IZ bolts GIVEN id = 'b1'").unwrap());
    assert_eq!(got[0][0], "41");

    alpha
        .execute("NAGRAD OBTAN NA COMRADE mill PREDEL bolts")
        .unwrap();
    let bilets = rows(alpha.execute("POKAZ BILET").unwrap());
    assert!(bilets.iter().any(|r| r[1] == "mill" && r[3] == "bolts"));
    alpha.execute("HELLO COMRADE mill").unwrap();
    alpha.execute("OBTAN * IZ bolts").unwrap();
    alpha.execute("HELLO COMRADE founder").unwrap();

    alpha.execute("USTANOV intensity = 25").unwrap();
    alpha.bureau.skip_sleep = true;
    alpha
        .execute("CONFISKAT TABL bolts SAMOKRIT 'audit'")
        .unwrap();
    delta.poll_mesh().unwrap();
    let err = delta.execute("OBTAN * IZ bolts").unwrap_err();
    assert_eq!(err.code, 1906);
    alpha.execute("OSVOBOD TABL bolts").unwrap();
    delta.poll_mesh().unwrap();
    assert_eq!(delta.execute("OBTAN * IZ bolts").unwrap().row_count(), 2);

    gamma.execute("USTANOV intensity = 25").unwrap();
    gamma.bureau.skip_sleep = true;
    gamma
        .execute("ACCUSE COMRADE mill OF SPY SAMOKRIT 'odd'")
        .unwrap();

    // Late joiner: empty disk, snapshot repair.
    let mut epsilon = plant(&hub, "epsilon");
    let snap = alpha.snapshot_msg();
    epsilon.apply_msg(snap).unwrap();
    let got = rows(epsilon.execute("OBTAN id IZ bolts LINEUP id").unwrap());
    assert_eq!(got.len(), 2);
    assert_eq!(got[0][0], "b1");

    let unsigned = epsilon
        .execute_unsigned("INZRT V bolts (id, plant, qty) ZNACH ('x', 'x', 1)")
        .unwrap_err();
    assert_eq!(unsigned.name, "UNSIGNED_MUTATION");
}

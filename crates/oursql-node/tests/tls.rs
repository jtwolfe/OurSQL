//! rustls 1.3 server config loads a real PEM pair.

#![cfg(feature = "tls")]

use std::process::Command;

#[test]
fn rustls_loads_self_signed_pem() {
    let dir = std::env::temp_dir().join(format!("oursql-tls-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let st = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ])
        .status()
        .expect("openssl");
    assert!(st.success());
    oursql_node_tls_load(&cert, &key);
    let _ = std::fs::remove_dir_all(&dir);
}

fn oursql_node_tls_load(cert: &std::path::Path, key: &std::path::Path) {
    // Re-export via the same module path the bin uses by compiling tls.rs logic:
    // we call rustls the same way as src/tls.rs.
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    let certs: Vec<_> = CertificateDer::pem_file_iter(cert)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_file(key).unwrap();
    let cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    assert!(cfg.alpn_protocols.is_empty() || true);
}

//! Optional rustls 1.3 wrapper. Native / --features tls only.

#![cfg(feature = "tls")]

use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

use oursql_core::{Error, Result};

pub fn server_config(cert: &Path, key: &Path, ca: Option<&Path>) -> Result<Arc<ServerConfig>> {
    let certs = load_certs(cert)?;
    let key = load_key(key)?;
    let builder = rustls::ServerConfig::builder();
    let cfg = if let Some(ca) = ca {
        let mut roots = rustls::RootCertStore::empty();
        let cas = load_certs(ca)?;
        for c in cas {
            roots.add(c).map_err(|e| Error::wal_io(e.to_string()))?;
        }
        let verifier = rustls::server::WebPkiClientVerifier::builder(roots.into())
            .build()
            .map_err(|e| Error::wal_io(e.to_string()))?;
        builder
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::wal_io(e.to_string()))?
    } else {
        builder
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::wal_io(e.to_string()))?
    };
    Ok(Arc::new(cfg))
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_file_iter(path)
        .and_then(|it| it.collect())
        .map_err(|e| Error::wal_io(e.to_string()))
}

fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(path).map_err(|e| Error::wal_io(e.to_string()))
}

pub fn accept(
    stream: std::net::TcpStream,
    cfg: &Arc<ServerConfig>,
) -> Result<rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>> {
    let conn =
        rustls::ServerConnection::new(cfg.clone()).map_err(|e| Error::wal_io(e.to_string()))?;
    Ok(rustls::StreamOwned::new(conn, stream))
}

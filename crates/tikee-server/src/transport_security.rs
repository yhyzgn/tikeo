//! TLS/mTLS listener configuration helpers.

use std::{fs::File, io::BufReader, sync::Arc};

use anyhow::{Context, Result, anyhow};
use rustls::{RootCertStore, ServerConfig, crypto::ring, pki_types::CertificateDer};
use tikee_config::TlsEndpointConfig;
use tonic::transport::{Certificate, Identity, ServerTlsConfig};

/// Build a Rustls server config for one endpoint.
///
/// # Errors
///
/// Returns an error when TLS is disabled, certificate/key files are missing or invalid,
/// or mTLS is required without a valid client CA bundle.
pub fn rustls_server_config(config: &TlsEndpointConfig) -> Result<ServerConfig> {
    if !config.tls_enabled {
        return Err(anyhow!("TLS is disabled for this endpoint"));
    }
    install_ring_provider();
    let certs = load_certs(required_path(config.cert_path.as_ref(), "cert_path")?)?;
    let key = load_private_key(required_path(config.key_path.as_ref(), "key_path")?)?;
    let builder = ServerConfig::builder();
    let server_config = if config.mtls_required {
        let roots = load_roots(required_path(
            config.client_ca_path.as_ref(),
            "client_ca_path",
        )?)?;
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .context("failed to build mTLS client certificate verifier")?;
        builder
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
    } else {
        builder.with_no_client_auth().with_single_cert(certs, key)
    }
    .context("failed to build TLS server certificate config")?;
    Ok(server_config)
}

/// Build a tonic gRPC server TLS config for the Worker Tunnel listener.
///
/// # Errors
///
/// Returns an error when TLS is disabled or configured certificate material cannot be read.
pub fn tonic_server_tls_config(config: &TlsEndpointConfig) -> Result<ServerTlsConfig> {
    if !config.tls_enabled {
        return Err(anyhow!("TLS is disabled for this endpoint"));
    }
    let cert = std::fs::read(required_path(config.cert_path.as_ref(), "cert_path")?)
        .context("failed to read server certificate")?;
    let key = std::fs::read(required_path(config.key_path.as_ref(), "key_path")?)
        .context("failed to read server private key")?;
    let mut tls = ServerTlsConfig::new().identity(Identity::from_pem(cert, key));
    if config.mtls_required {
        let ca = std::fs::read(required_path(
            config.client_ca_path.as_ref(),
            "client_ca_path",
        )?)
        .context("failed to read client CA bundle")?;
        tls = tls.client_ca_root(Certificate::from_pem(ca));
    }
    Ok(tls)
}

fn install_ring_provider() {
    let _ = ring::default_provider().install_default();
}

fn required_path<'a>(value: Option<&'a String>, name: &str) -> Result<&'a str> {
    value
        .map(String::as_str)
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required when TLS is enabled"))
}

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path).with_context(|| format!("open cert {path}"))?);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parse cert bundle {path}"))?;
    if certs.is_empty() {
        return Err(anyhow!("certificate bundle {path} is empty"));
    }
    Ok(certs)
}

fn load_private_key(path: &str) -> Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(File::open(path).with_context(|| format!("open key {path}"))?);
    rustls_pemfile::private_key(&mut reader)
        .with_context(|| format!("parse private key {path}"))?
        .ok_or_else(|| anyhow!("private key {path} is empty"))
}

fn load_roots(path: &str) -> Result<RootCertStore> {
    let certs = load_certs(path)?;
    let mut roots = RootCertStore::empty();
    for cert in certs {
        roots
            .add(cert)
            .with_context(|| format!("add client CA certificate from {path}"))?;
    }
    Ok(roots)
}

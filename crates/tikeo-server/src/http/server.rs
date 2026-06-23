//! HTTP listener serving helpers.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;
use tokio::{net::TcpListener, signal};
use tokio_rustls::TlsAcceptor;
use tracing::info;

use super::router::router_for_database;

/// Run the unified HTTP listener.
///
/// # Errors
///
/// Returns an error when binding the configured listener address, initializing storage,
/// or serving HTTP fails.
pub async fn serve(listen_addr: SocketAddr, connection_url: &str) -> Result<()> {
    serve_with_state(listen_addr, router_for_database(connection_url).await?).await
}

/// Run the unified HTTP listener with prebuilt application state.
///
/// # Errors
///
/// Returns an error when binding the configured listener address or serving HTTP fails.
pub async fn serve_with_state(listen_addr: SocketAddr, router: Router) -> Result<()> {
    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind {listen_addr}"))?;
    serve_listener_with_state(
        listener,
        router,
        &tikeo_config::TlsEndpointConfig::default(),
    )
    .await
}

/// Run the HTTP listener using an already-bound socket and endpoint TLS config.
///
/// # Errors
///
/// Returns an error when TLS material cannot be loaded or serving HTTP fails.
pub async fn serve_listener_with_state(
    listener: TcpListener,
    router: Router,
    tls: &tikeo_config::TlsEndpointConfig,
) -> Result<()> {
    let listen_addr = listener
        .local_addr()
        .context("failed to read HTTP listener local address")?;

    info!(addr = %listen_addr, "tikeo HTTP server listening");

    if tls.tls_enabled {
        serve_tls_listener(listener, router, tls).await
    } else {
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("tikeo HTTP server failed")
    }
}

async fn serve_tls_listener(
    listener: TcpListener,
    router: Router,
    tls: &tikeo_config::TlsEndpointConfig,
) -> Result<()> {
    // Validate once at startup, then rebuild the acceptor for each new connection so
    // certificate/key/CA file rotations are picked up without restarting the process.
    crate::transport_security::rustls_server_config(tls)?;
    let tls = tls.clone();
    loop {
        tokio::select! {
            () = shutdown_signal() => return Ok(()),
            accepted = listener.accept() => {
                let (stream, _peer_addr) = accepted.context("failed to accept HTTP TLS connection")?;
                let router = router.clone();
                let tls = tls.clone();
                tokio::spawn(async move {
                    if let Err(error) = serve_tls_connection(stream, &tls, router).await {
                        tracing::warn!(%error, "HTTP TLS connection failed");
                    }
                });
            }
        }
    }
}

async fn serve_tls_connection(
    stream: tokio::net::TcpStream,
    tls: &tikeo_config::TlsEndpointConfig,
    router: Router,
) -> Result<()> {
    let config = crate::transport_security::rustls_server_config(tls)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let tls_stream = acceptor
        .accept(stream)
        .await
        .context("failed to accept HTTP TLS handshake")?;
    let io = hyper_util::rt::TokioIo::new(tls_stream);
    let service = hyper_util::service::TowerToHyperService::new(router);
    hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
        .serve_connection_with_upgrades(io, service)
        .await
        .map_err(|error| anyhow::anyhow!("failed to serve HTTP TLS connection: {error}"))
}

async fn shutdown_signal() {
    if let Err(error) = signal::ctrl_c().await {
        tracing::warn!(%error, "failed to listen for shutdown signal");
    }
}

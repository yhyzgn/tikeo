//! HTTP management gateway for tikeo.

/// Access scope module.
pub mod access_scope;
/// `Auth` module.
pub mod auth;
/// `Dto` module.
pub mod dto;
/// `Error` module.
pub mod error;
mod health;
/// `Oidc` module.
pub mod oidc;
mod oidc_session;
mod opaque_token;
/// `Openapi` module.
pub mod openapi;
mod router;
/// `Routes` module.
pub mod routes;
mod sdk_api_keys;
mod server;
/// `Services` module.
pub mod services;
/// `Session` module.
pub mod session;
mod session_metadata;
mod state;
/// `Trace` module.
pub mod trace;

pub use self::{
    router::router_with_state,
    server::{serve, serve_listener_with_state, serve_with_state},
    state::{AppState, AppStateParts},
};

#[cfg(test)]
mod tests;

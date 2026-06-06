//! HTTP management gateway for tikee.

pub mod access_scope;
pub mod auth;
pub mod dto;
pub mod error;
mod health;
pub mod oidc;
mod oidc_session;
mod opaque_token;
pub mod openapi;
mod router;
pub mod routes;
mod sdk_api_keys;
mod server;
pub mod services;
pub mod session;
mod session_metadata;
mod state;
pub mod trace;

pub use self::{
    router::router_with_state,
    server::{serve, serve_listener_with_state, serve_with_state},
    state::AppState,
};

#[cfg(test)]
mod tests;

//! Shared opaque-token generation helpers.
//!
//! Bearer credentials in the HTTP layer are random, database-backed secrets.
//! They intentionally carry no embedded claims and must not become JWTs.

use rand::{TryRngCore, rngs::OsRng};

use super::error::ApiError;

const BASE62: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const REJECTION_LIMIT: u8 = 248;

/// Generate a cryptographically random base62 secret with exact character length.
///
/// # Errors
///
/// Returns an API error when the OS random source cannot provide secure bytes.
pub fn generate_base62(length: usize) -> Result<String, ApiError> {
    let mut random = String::with_capacity(length);
    let mut byte = [0_u8; 1];
    while random.len() < length {
        OsRng
            .try_fill_bytes(&mut byte)
            .map_err(|error| ApiError::bad_request(format!("os rng failed: {error}")))?;
        if byte[0] < REJECTION_LIMIT {
            random.push(BASE62[usize::from(byte[0] % 62)] as char);
        }
    }
    Ok(random)
}

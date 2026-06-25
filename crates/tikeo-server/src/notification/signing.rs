//! Provider-specific request signing for built-in notification channels.

use std::fmt::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Return a `DingTalk` custom-robot URL with `timestamp` and `sign` query params.
#[must_use]
/// Signed dingtalk url.
pub(super) fn signed_dingtalk_url(url: &str, secret: &str) -> String {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
    let string_to_sign = format!("{timestamp}\n{secret}");
    let sign = STANDARD.encode(hmac_sha256(secret.as_bytes(), string_to_sign.as_bytes()));
    append_query_params(
        url,
        &[
            ("timestamp", timestamp.to_string()),
            ("sign", percent_encode(&sign)),
        ],
    )
}

/// Add Feishu/Lark custom-bot `timestamp` and `sign` fields to the JSON body.
pub(super) fn add_feishu_signature(body: &mut serde_json::Value, secret: &str) {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let string_to_sign = format!("{timestamp}\n{secret}");
    let sign = STANDARD.encode(hmac_sha256(string_to_sign.as_bytes(), &[]));
    if let Some(object) = body.as_object_mut() {
        object.insert("timestamp".to_owned(), serde_json::Value::String(timestamp));
        object.insert("sign".to_owned(), serde_json::Value::String(sign));
    }
}

fn append_query_params(url: &str, params: &[(&str, String)]) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    let rendered = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{url}{separator}{rendered}")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(&mut encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        key_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut outer = [0x5c_u8; BLOCK_SIZE];
    let mut inner = [0x36_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        outer[index] ^= key_block[index];
        inner[index] ^= key_block[index];
    }
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(inner);
    inner_hasher.update(message);
    let inner_hash = inner_hasher.finalize();
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(outer);
    outer_hasher.update(inner_hash);
    outer_hasher.finalize().into()
}

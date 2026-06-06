//! SMTP email alert delivery helpers.

use super::{AlertDeliveryPolicy, AlertDeliveryResult, AlertPayload, Severity, is_loopback_host};
use rustls::pki_types::ServerName;
use serde::Deserialize;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::{TlsConnector, client::TlsStream};
use url::Url;

pub async fn deliver_email_channel(
    smtp_url: &str,
    from: &str,
    recipients: &[String],
    username: Option<&str>,
    password: Option<&str>,
    payload: &AlertPayload,
    policy: AlertDeliveryPolicy,
) -> AlertDeliveryResult {
    let target = recipients.join(",");
    let result = send_plain_smtp(
        smtp_url, from, recipients, username, password, payload, policy,
    )
    .await;
    match result {
        Ok(()) => AlertDeliveryResult {
            provider: "email".to_owned(),
            target,
            delivered: true,
            status: None,
            error: None,
        },
        Err(error) => AlertDeliveryResult {
            provider: "email".to_owned(),
            target,
            delivered: false,
            status: None,
            error: Some(error),
        },
    }
}

async fn send_plain_smtp(
    smtp_url: &str,
    from: &str,
    recipients: &[String],
    username: Option<&str>,
    password: Option<&str>,
    payload: &AlertPayload,
    policy: AlertDeliveryPolicy,
) -> Result<(), String> {
    let parsed = validate_smtp_url(smtp_url, policy)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "smtp url must include host".to_owned())?;
    let port = parsed
        .port()
        .unwrap_or_else(|| if parsed.scheme() == "smtps" { 465 } else { 25 });
    if parsed.scheme() == "smtps" {
        let tcp = connect_smtp_tcp(host, port).await?;
        let mut stream = connect_smtp_tls(tcp, host).await?;
        smtp_handshake_and_auth(&mut stream, username, password).await?;
        write_envelope_and_data(&mut stream, from, recipients, payload).await?;
        write_smtp_command(&mut stream, "QUIT").await?;
        let _ = read_smtp_response(&mut stream).await;
        return Ok(());
    }
    let mut stream = connect_smtp_tcp(host, port).await?;
    read_smtp_response(&mut stream).await?;
    write_smtp_command(&mut stream, "EHLO tikeo").await?;
    read_smtp_response(&mut stream).await?;
    if parsed.scheme() == "smtp+starttls" {
        write_smtp_command(&mut stream, "STARTTLS").await?;
        read_smtp_response(&mut stream).await?;
        let mut stream = connect_smtp_tls(stream, host).await?;
        smtp_handshake_and_auth(&mut stream, username, password).await?;
        write_envelope_and_data(&mut stream, from, recipients, payload).await?;
        write_smtp_command(&mut stream, "QUIT").await?;
        let _ = read_smtp_response(&mut stream).await;
        return Ok(());
    }
    smtp_auth(&mut stream, username, password).await?;
    write_envelope_and_data(&mut stream, from, recipients, payload).await?;
    write_smtp_command(&mut stream, "QUIT").await?;
    let _ = read_smtp_response(&mut stream).await;
    Ok(())
}

async fn connect_smtp_tcp(host: &str, port: u16) -> Result<TcpStream, String> {
    TcpStream::connect((host, port))
        .await
        .map_err(|error| format!("smtp connect failed: {error}"))
}

async fn connect_smtp_tls(stream: TcpStream, host: &str) -> Result<TlsStream<TcpStream>, String> {
    let mut roots = rustls::RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs();
    if !native_certs.errors.is_empty() {
        return Err("smtp tls native cert load failed".to_owned());
    }
    for cert in native_certs.certs {
        roots
            .add(cert)
            .map_err(|error| format!("smtp tls root cert load failed: {error}"))?;
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let server_name = ServerName::try_from(host.to_owned())
        .map_err(|_| "smtp tls server name is invalid".to_owned())?;
    TlsConnector::from(Arc::new(config))
        .connect(server_name, stream)
        .await
        .map_err(|error| format!("smtp tls handshake failed: {error}"))
}

async fn smtp_handshake_and_auth<S>(
    stream: &mut S,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(), String>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    read_smtp_response(stream).await?;
    write_smtp_command(stream, "EHLO tikeo").await?;
    read_smtp_response(stream).await?;
    smtp_auth(stream, username, password).await
}

async fn smtp_auth<S>(
    stream: &mut S,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(), String>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    if let Some(username) = username {
        let password = password.ok_or_else(|| {
            "smtp password secret ref is required when username is configured".to_owned()
        })?;
        write_smtp_command(stream, "AUTH LOGIN").await?;
        read_smtp_response(stream).await?;
        write_smtp_command(stream, &base64_encode(username)).await?;
        read_smtp_response(stream).await?;
        write_smtp_command(stream, &base64_encode(password)).await?;
        read_smtp_response(stream).await?;
    }
    Ok(())
}

async fn write_envelope_and_data<S>(
    stream: &mut S,
    from: &str,
    recipients: &[String],
    payload: &AlertPayload,
) -> Result<(), String>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    write_smtp_command(stream, &format!("MAIL FROM:<{from}>")).await?;
    read_smtp_response(stream).await?;
    for recipient in recipients {
        write_smtp_command(stream, &format!("RCPT TO:<{recipient}>")).await?;
        read_smtp_response(stream).await?;
    }
    write_smtp_command(stream, "DATA").await?;
    read_smtp_response(stream).await?;
    let subject = format!(
        "[tikeo/{}] {}",
        severity_label(&payload.severity),
        payload.rule_name
    );
    let body = format!(
        "From: {from}\r\nTo: {}\r\nSubject: {subject}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\nResource: {}/{}\r\nTriggered-At: {}\r\n.",
        recipients.join(", "),
        payload.message,
        payload.resource_type,
        payload.resource_id,
        payload.triggered_at,
    );
    write_smtp_command(stream, &body).await?;
    read_smtp_response(stream).await?;
    Ok(())
}

fn validate_smtp_url(value: &str, policy: AlertDeliveryPolicy) -> Result<Url, String> {
    let parsed = Url::parse(value).map_err(|_| "invalid smtp url".to_owned())?;
    let Some(host) = parsed.host_str() else {
        return Err("smtp url must include host".to_owned());
    };
    let host_lower = host.to_ascii_lowercase();
    match parsed.scheme() {
        "smtp" if policy.allow_insecure_loopback && is_loopback_host(&host_lower) => Ok(parsed),
        "smtps" | "smtp+starttls" => Ok(parsed),
        "smtp" => Err("plain smtp is allowed only for explicit loopback smoke tests".to_owned()),
        _ => Err("smtp url must use smtps://, smtp+starttls://, or loopback smtp://".to_owned()),
    }
}

fn base64_encode(value: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(value.as_bytes())
}

async fn write_smtp_command<S>(stream: &mut S, command: &str) -> Result<(), String>
where
    S: AsyncWriteExt + Unpin,
{
    stream
        .write_all(command.as_bytes())
        .await
        .map_err(|error| format!("smtp write failed: {error}"))?;
    stream
        .write_all(b"\r\n")
        .await
        .map_err(|error| format!("smtp write failed: {error}"))
}

async fn read_smtp_response<S>(stream: &mut S) -> Result<String, String>
where
    S: AsyncReadExt + Unpin,
{
    let mut buffer = [0_u8; 1024];
    let read = stream
        .read(&mut buffer)
        .await
        .map_err(|error| format!("smtp read failed: {error}"))?;
    if read == 0 {
        return Err("smtp server closed connection".to_owned());
    }
    let response = String::from_utf8_lossy(&buffer[..read]).to_string();
    if response.starts_with('4') || response.starts_with('5') {
        return Err(format!("smtp rejected command: {}", response.trim()));
    }
    Ok(response)
}

const fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

pub(super) fn deserialize_recipients<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(item) => Ok(vec![item]),
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                serde_json::Value::String(value) => Ok(value),
                _ => Err(serde::de::Error::custom(
                    "recipient entries must be strings",
                )),
            })
            .collect(),
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(serde::de::Error::custom(
            "recipients must be a string or array",
        )),
    }
}

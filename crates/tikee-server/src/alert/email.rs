//! SMTP email alert delivery helpers.

use super::{AlertDeliveryPolicy, AlertDeliveryResult, AlertPayload, Severity, is_loopback_host};
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::Url;

pub async fn deliver_email_channel(
    smtp_url: &str,
    from: &str,
    recipients: &[String],
    payload: &AlertPayload,
    policy: AlertDeliveryPolicy,
) -> AlertDeliveryResult {
    let target = recipients.join(",");
    let result = send_plain_smtp(smtp_url, from, recipients, payload, policy).await;
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
    payload: &AlertPayload,
    policy: AlertDeliveryPolicy,
) -> Result<(), String> {
    let parsed = validate_smtp_url(smtp_url, policy)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "smtp url must include host".to_owned())?;
    let port = parsed.port().unwrap_or(25);
    let mut stream = TcpStream::connect((host, port))
        .await
        .map_err(|error| format!("smtp connect failed: {error}"))?;
    read_smtp_response(&mut stream).await?;
    write_smtp_command(&mut stream, "EHLO tikee").await?;
    read_smtp_response(&mut stream).await?;
    write_smtp_command(&mut stream, &format!("MAIL FROM:<{from}>")).await?;
    read_smtp_response(&mut stream).await?;
    for recipient in recipients {
        write_smtp_command(&mut stream, &format!("RCPT TO:<{recipient}>")).await?;
        read_smtp_response(&mut stream).await?;
    }
    write_smtp_command(&mut stream, "DATA").await?;
    read_smtp_response(&mut stream).await?;
    let subject = format!(
        "[tikee/{}] {}",
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
    write_smtp_command(&mut stream, &body).await?;
    read_smtp_response(&mut stream).await?;
    write_smtp_command(&mut stream, "QUIT").await?;
    let _ = read_smtp_response(&mut stream).await;
    Ok(())
}

fn validate_smtp_url(value: &str, policy: AlertDeliveryPolicy) -> Result<Url, String> {
    let parsed = Url::parse(value).map_err(|_| "invalid smtp url".to_owned())?;
    let Some(host) = parsed.host_str() else {
        return Err("smtp url must include host".to_owned());
    };
    let host_lower = host.to_ascii_lowercase();
    if parsed.scheme() == "smtp" && policy.allow_insecure_loopback && is_loopback_host(&host_lower)
    {
        return Ok(parsed);
    }
    Err("smtp delivery currently requires explicit local loopback smtp:// policy".to_owned())
}

async fn write_smtp_command(stream: &mut TcpStream, command: &str) -> Result<(), String> {
    stream
        .write_all(command.as_bytes())
        .await
        .map_err(|error| format!("smtp write failed: {error}"))?;
    stream
        .write_all(b"\r\n")
        .await
        .map_err(|error| format!("smtp write failed: {error}"))
}

async fn read_smtp_response(stream: &mut TcpStream) -> Result<String, String> {
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

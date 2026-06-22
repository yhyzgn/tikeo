use anyhow::{Context, Result};

use crate::{ApplyCommand, ApplyEvidence, ApplyRequestEvidence, MigrationReport};

pub(crate) async fn apply_data(
    command: &ApplyCommand,
    report: &MigrationReport,
) -> Result<ApplyEvidence> {
    let mut drafts = Vec::new();
    for job in &report.jobs {
        if job.status == "ready" || (command.include_needs_review && job.status == "needs_review") {
            if let Some(draft) = &job.tikeo_job {
                drafts.push(draft.clone());
            }
        }
    }
    let mut requests = Vec::new();
    if command.dry_run {
        for draft in drafts {
            requests.push(ApplyRequestEvidence {
                name: draft.name,
                status: "planned".to_owned(),
                http_status: None,
                response: None,
            });
        }
        return Ok(ApplyEvidence {
            dry_run: true,
            requests,
        });
    }
    let client = reqwest::Client::new();
    let endpoint = command.endpoint.trim_end_matches('/');
    for draft in drafts {
        let response = client
            .post(format!("{endpoint}/api/v1/jobs"))
            .header("x-tikeo-api-key", &command.api_key)
            .json(&draft)
            .send()
            .await
            .with_context(|| format!("failed to apply job {}", draft.name))?;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        requests.push(ApplyRequestEvidence {
            name: draft.name,
            status: if (200..300).contains(&status) {
                "applied"
            } else {
                "failed"
            }
            .to_owned(),
            http_status: Some(status),
            response: Some(body),
        });
    }
    Ok(ApplyEvidence {
        dry_run: false,
        requests,
    })
}

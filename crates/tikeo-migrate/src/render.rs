use std::fmt::Write as _;

use crate::{JavaProjectMigrationPlan, MigrationBundle, MigrationReport};

/// Render checklist.
pub fn render_checklist(bundle: &MigrationBundle) -> String {
    let mut output = format!(
        "# Tikeo migration checklist\n\nSource: `{}`\n\n",
        bundle.source
    );
    for (index, item) in bundle.checklist.iter().enumerate() {
        let _ = writeln!(output, "{}. {item}", index + 1);
    }
    output
}

/// Render java project plan.
pub fn render_java_project_plan(project: &JavaProjectMigrationPlan) -> String {
    let mut output = format!(
        "# Java project migration plan\n\n- Project: `{}`\n- Build system: `{}`\n- Spring Boot major: `{}`\n- Recommended artifact: `net.tikeo:{}`\n\n## Dependency\n\n```\n{}\n```\n\n",
        project.project_root,
        project.build_system,
        project
            .spring_boot_major
            .map_or_else(|| "unknown".to_owned(), |major| major.to_string()),
        project.recommended_artifact,
        project.dependency_snippet
    );
    output.push_str("## Handler candidates\n\n");
    for handler in &project.handler_candidates {
        let _ = writeln!(
            output,
            "- `{}` → `{}` ({})",
            handler.path, handler.processor_name, handler.framework
        );
    }
    if project.handler_candidates.is_empty() {
        output.push_str("- No legacy handlers detected.\n");
    }
    output.push_str("\n## Review notes\n\n");
    for note in &project.review_notes {
        let _ = writeln!(output, "- {note}");
    }
    output
}

/// Render markdown report.
pub fn render_markdown_report(report: &MigrationReport) -> String {
    let mut output = format!(
        "# Tikeo migration report\n\n- Source: `{}`\n- Mode: `{}`\n- Total: {}\n- Ready: {}\n- Needs review: {}\n- Skipped: {}\n\n",
        report.source,
        report.mode,
        report.summary.total,
        report.summary.ready,
        report.summary.needs_review,
        report.summary.skipped
    );
    output.push_str("| Source ID | Name | Status | Tikeo schedule | Processor | Notes |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for job in &report.jobs {
        let (schedule, processor) = job.tikeo_job.as_ref().map_or(("-", "-"), |draft| {
            (
                draft.schedule_type.as_str(),
                draft.processor_name.as_deref().unwrap_or("-"),
            )
        });
        let notes = job
            .unsupported_features
            .iter()
            .chain(job.warnings.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("; ")
            .replace('|', "\\|");
        let _ = writeln!(
            output,
            "| `{}` | {} | {} | `{}` | `{}` | {} |",
            job.source_id,
            job.source_name.replace('|', "\\|"),
            job.status,
            schedule,
            processor,
            notes
        );
    }
    output
}

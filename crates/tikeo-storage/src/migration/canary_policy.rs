use super::{DatabaseBackend, Jobs, Statement, text_col};
use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

pub(super) struct CanaryPolicyMigration;

impl MigrationName for CanaryPolicyMigration {
    fn name(&self) -> &'static str {
        "m20260617_000002_canary_policy"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CanaryPolicyMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let default_json = tikeo_storage_policy_default().replace('\'', "''");
        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            if !sqlite_table_exists(manager, "jobs").await? {
                return Ok(());
            }
            if !sqlite_column_exists(manager, "jobs", "canary_policy_json").await? {
                manager
                    .get_connection()
                    .execute(Statement::from_string(
                        DatabaseBackend::Sqlite,
                        format!(
                            "ALTER TABLE jobs ADD COLUMN canary_policy_json text NOT NULL DEFAULT '{default_json}'"
                        ),
                    ))
                    .await?;
            }
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column_if_not_exists(
                        text_col(Jobs::CanaryPolicyJson).default(tikeo_storage_policy_default()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

fn tikeo_storage_policy_default() -> String {
    crate::repository::JobCanaryPolicy::default_json()
}

async fn sqlite_column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA table_info({table})"),
        ))
        .await?;
    for row in rows {
        let name: String = row.try_get("", "name")?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn sqlite_table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
        ))
        .await?;
    Ok(!rows.is_empty())
}

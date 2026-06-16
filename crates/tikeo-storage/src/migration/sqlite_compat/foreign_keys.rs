use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

pub(super) async fn remove_sqlite_foreign_keys(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    remove_sqlite_scope_foreign_keys(db).await?;
    remove_sqlite_job_foreign_keys(db).await?;
    remove_sqlite_auth_foreign_keys(db).await?;
    ensure_sqlite_indexes(db).await
}

async fn remove_sqlite_scope_foreign_keys(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "apps",
        r"CREATE TABLE apps (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &["id", "namespace_id", "name", "created_at", "updated_at"],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "worker_pools",
        r"CREATE TABLE worker_pools (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "namespace_id",
            "app_id",
            "name",
            "created_at",
            "updated_at",
        ],
    )
    .await
}

async fn remove_sqlite_job_foreign_keys(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "jobs",
        r"CREATE TABLE jobs (
            id varchar NOT NULL PRIMARY KEY,
            namespace_id varchar NOT NULL,
            app_id varchar NOT NULL,
            name varchar NOT NULL,
            schedule_type varchar NOT NULL,
            schedule_expr varchar,
            misfire_policy varchar NOT NULL DEFAULT 'fire_once',
            schedule_start_at varchar,
            schedule_end_at varchar,
            schedule_calendar_json varchar,
            processor_name varchar,
            processor_type varchar,
            script_id varchar,
            enabled boolean NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "namespace_id",
            "app_id",
            "name",
            "schedule_type",
            "schedule_expr",
            "processor_name",
            "processor_type",
            "script_id",
            "enabled",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instances",
        r"CREATE TABLE job_instances (
            id varchar NOT NULL PRIMARY KEY,
            job_id varchar NOT NULL,
            status varchar NOT NULL,
            trigger_type varchar NOT NULL,
            execution_mode varchar NOT NULL DEFAULT 'single',
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "job_id",
            "status",
            "trigger_type",
            "execution_mode",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    remove_sqlite_job_observability_foreign_keys(db).await
}

async fn remove_sqlite_job_observability_foreign_keys(
    db: &impl ConnectionTrait,
) -> Result<(), sea_orm::DbErr> {
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instance_attempts",
        r"CREATE TABLE job_instance_attempts (
            id varchar NOT NULL PRIMARY KEY,
            instance_id varchar NOT NULL,
            worker_id varchar NOT NULL,
            status varchar NOT NULL,
            assignment_token varchar,
            result_success boolean,
            result_message text,
            result_completed_at varchar,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "instance_id",
            "worker_id",
            "status",
            "assignment_token",
            "result_success",
            "result_message",
            "result_completed_at",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "job_instance_logs",
        r"CREATE TABLE job_instance_logs (
            id varchar NOT NULL PRIMARY KEY,
            instance_id varchar NOT NULL,
            worker_id varchar NOT NULL,
            level varchar NOT NULL,
            message varchar NOT NULL,
            sequence bigint NOT NULL,
            created_at varchar NOT NULL
        )",
        &[
            "id",
            "instance_id",
            "worker_id",
            "level",
            "message",
            "sequence",
            "created_at",
        ],
    )
    .await
}

async fn remove_sqlite_auth_foreign_keys(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
    rebuild_sqlite_table_without_foreign_keys(
        db,
        "auth_sessions",
        r"CREATE TABLE auth_sessions (
            id varchar NOT NULL PRIMARY KEY,
            user_id varchar NOT NULL,
            token_hash varchar NOT NULL,
            device_id varchar,
            device_name varchar,
            expires_at varchar NOT NULL,
            created_at varchar NOT NULL,
            updated_at varchar NOT NULL
        )",
        &[
            "id",
            "user_id",
            "token_hash",
            "device_id",
            "device_name",
            "expires_at",
            "created_at",
            "updated_at",
        ],
    )
    .await
}

async fn rebuild_sqlite_table_without_foreign_keys(
    db: &impl ConnectionTrait,
    table: &str,
    create_sql: &str,
    columns: &[&str],
) -> Result<(), sea_orm::DbErr> {
    if !sqlite_table_has_foreign_keys(db, table).await? {
        return Ok(());
    }

    let backup = format!("{table}__soft_rel_tmp");
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys=OFF",
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("ALTER TABLE {table} RENAME TO {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        create_sql.to_owned(),
    ))
    .await?;
    let column_list = columns.join(", ");
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("INSERT INTO {table} ({column_list}) SELECT {column_list} FROM {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("DROP TABLE {backup}"),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys=ON",
    ))
    .await?;
    Ok(())
}

async fn sqlite_table_has_foreign_keys(
    db: &impl ConnectionTrait,
    table: &str,
) -> Result<bool, sea_orm::DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA foreign_key_list({table})"),
        ))
        .await?;
    Ok(!rows.is_empty())
}

async fn ensure_sqlite_indexes(db: &impl ConnectionTrait) -> Result<(), sea_orm::DbErr> {
    for sql in [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_apps_namespace_name ON apps (namespace_id, name)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_pools_app_name ON worker_pools (app_id, name)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_app_name ON jobs (app_id, name)",
        "CREATE INDEX IF NOT EXISTS idx_jobs_enabled ON jobs (enabled)",
        "CREATE INDEX IF NOT EXISTS idx_job_instances_job_created ON job_instances (job_id, created_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_job_instance_attempts_instance_worker ON job_instance_attempts (instance_id, worker_id)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_attempts_status ON job_instance_attempts (status)",
        "CREATE INDEX IF NOT EXISTS idx_job_instance_logs_instance_seq ON job_instance_logs (instance_id, sequence)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_token_hash ON auth_sessions (token_hash)",
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions (user_id)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_auth_states_state_hash ON oidc_auth_states (state_hash)",
        "CREATE INDEX IF NOT EXISTS idx_oidc_auth_states_expires ON oidc_auth_states (expires_at)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_oidc_identities_issuer_subject ON oidc_identities (issuer, subject)",
        "CREATE INDEX IF NOT EXISTS idx_oidc_identities_username ON oidc_identities (username)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_logical_instances_key ON worker_logical_instances (namespace_name, app_name, cluster, region, client_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_worker_sessions_status_lease ON worker_sessions (status, lease_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_worker_sessions_logical_generation ON worker_sessions (logical_instance_id, generation)",
        "CREATE INDEX IF NOT EXISTS idx_worker_session_events_worker_created ON worker_session_events (worker_id, created_at)",
    ] {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await?;
    }
    Ok(())
}

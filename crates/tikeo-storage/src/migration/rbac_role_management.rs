use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::{
    DbErr, IntoIden, MigrationName, MigrationTrait, SchemaManager, Table, async_trait, sea_query,
};
use sea_query::Index;

use super::{
    DatabaseBackend, Permissions, RoleMenuPermissions, RoleUiActionPermissions, Roles, Statement,
    UserRoles, create_rbac_tables, create_users, drop_tables, exec_seed_insert_if_missing,
    now_rfc3339, seed_rbac_defaults, seed_role_permissions, string_col, string_pk,
};

pub(super) struct RbacRoleManagementMigration;

impl MigrationName for RbacRoleManagementMigration {
    fn name(&self) -> &'static str {
        "m20260607_000001_rbac_role_management"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for RbacRoleManagementMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rbac_role_management_dependencies(manager).await?;
        add_rbac_role_management_columns(manager).await?;
        create_role_management_tables(manager).await?;
        create_role_management_indexes(manager).await?;
        seed_role_management_defaults(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_tables(
            manager,
            &[
                RoleUiActionPermissions::Table.into_iden(),
                RoleMenuPermissions::Table.into_iden(),
                UserRoles::Table.into_iden(),
            ],
        )
        .await
    }
}

async fn ensure_rbac_role_management_dependencies(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    create_users(manager).await?;
    create_rbac_tables(manager).await?;
    seed_rbac_defaults(manager).await
}

async fn add_rbac_role_management_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let _ = (
        Roles::DisplayName,
        Roles::Builtin,
        Roles::Enabled,
        Roles::UpdatedAt,
    );
    for (column, definition) in [
        ("display_name", "varchar(191) NOT NULL DEFAULT ''"),
        ("builtin", "boolean NOT NULL DEFAULT FALSE"),
        ("enabled", "boolean NOT NULL DEFAULT TRUE"),
        ("updated_at", "varchar(191) NOT NULL DEFAULT ''"),
    ] {
        if manager.get_database_backend() == DatabaseBackend::Sqlite
            && sqlite_column_exists(manager, "roles", column).await?
        {
            continue;
        }
        exec_sql(
            manager,
            &format!("ALTER TABLE roles ADD COLUMN {column} {definition}"),
        )
        .await?;
    }
    Ok(())
}

async fn create_role_management_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserRoles::Table)
                .if_not_exists()
                .col(string_pk(UserRoles::Id))
                .col(string_col(UserRoles::UserId))
                .col(string_col(UserRoles::RoleId))
                .col(string_col(UserRoles::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RoleMenuPermissions::Table)
                .if_not_exists()
                .col(string_pk(RoleMenuPermissions::Id))
                .col(string_col(RoleMenuPermissions::RoleId))
                .col(string_col(RoleMenuPermissions::MenuKey))
                .col(string_col(RoleMenuPermissions::CreatedAt))
                .to_owned(),
        )
        .await?;
    manager
        .create_table(
            Table::create()
                .table(RoleUiActionPermissions::Table)
                .if_not_exists()
                .col(string_pk(RoleUiActionPermissions::Id))
                .col(string_col(RoleUiActionPermissions::RoleId))
                .col(string_col(RoleUiActionPermissions::UiActionKey))
                .col(string_col(RoleUiActionPermissions::CreatedAt))
                .to_owned(),
        )
        .await
}

async fn create_role_management_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_user_roles_user_role")
                .table(UserRoles::Table)
                .col(UserRoles::UserId)
                .col(UserRoles::RoleId)
                .if_not_exists()
                .unique()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_role_menu_permissions_role_menu")
                .table(RoleMenuPermissions::Table)
                .col(RoleMenuPermissions::RoleId)
                .col(RoleMenuPermissions::MenuKey)
                .if_not_exists()
                .unique()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_role_ui_action_permissions_role_action")
                .table(RoleUiActionPermissions::Table)
                .col(RoleUiActionPermissions::RoleId)
                .col(RoleUiActionPermissions::UiActionKey)
                .if_not_exists()
                .unique()
                .to_owned(),
        )
        .await
}

async fn seed_role_management_defaults(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now = now_rfc3339();
    exec_sql(
        manager,
        &format!(
            "UPDATE roles SET display_name = name, updated_at = '{now}', enabled = TRUE, builtin = CASE WHEN name = 'owner' THEN TRUE ELSE FALSE END WHERE display_name = '' OR updated_at = ''"
        ),
    )
    .await?;

    for (id, resource, action, description) in ROLE_MANAGEMENT_PERMISSIONS {
        let insert = sea_query::Query::insert()
            .into_table(Permissions::Table)
            .columns([
                Permissions::Id,
                Permissions::Resource,
                Permissions::Action,
                Permissions::Description,
                Permissions::CreatedAt,
            ])
            .values_panic([
                (*id).into(),
                (*resource).into(),
                (*action).into(),
                (*description).into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "permissions", id, insert).await?;
    }
    seed_role_permissions(
        manager,
        "role-owner",
        ["perm-roles-read", "perm-roles-manage", "perm-roles-assign"],
    )
    .await?;

    backfill_user_roles(manager).await?;
    seed_role_menu_permissions(manager, "role-owner", ALL_MENU_KEYS).await?;
    seed_role_menu_permissions(manager, "role-operator", OPERATOR_MENU_KEYS).await?;
    seed_role_menu_permissions(manager, "role-viewer", VIEWER_MENU_KEYS).await?;
    seed_role_ui_action_permissions(manager, "role-owner", ALL_UI_ACTION_KEYS).await?;
    seed_role_ui_action_permissions(manager, "role-operator", OPERATOR_UI_ACTION_KEYS).await?;
    seed_role_ui_action_permissions(manager, "role-viewer", VIEWER_UI_ACTION_KEYS).await
}

async fn backfill_user_roles(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DatabaseBackend::Sqlite
        && !sqlite_table_exists(manager, "users").await?
    {
        return Ok(());
    }
    let now = now_rfc3339();
    exec_sql(
        manager,
        &format!(
            "INSERT INTO user_roles (id, user_id, role_id, created_at) SELECT 'ur-' || users.id || '-' || roles.id, users.id, roles.id, '{now}' FROM users JOIN roles ON roles.name = users.role WHERE NOT EXISTS (SELECT 1 FROM user_roles existing WHERE existing.user_id = users.id AND existing.role_id = roles.id)"
        ),
    )
    .await
}

async fn seed_role_menu_permissions<I, S>(
    manager: &SchemaManager<'_>,
    role_id: &str,
    menu_keys: I,
) -> Result<(), DbErr>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for menu_key in menu_keys {
        let menu_key = menu_key.as_ref();
        let binding_id = format!("rmp-{role_id}-{menu_key}").replace('/', "_");
        let insert = sea_query::Query::insert()
            .into_table(RoleMenuPermissions::Table)
            .columns([
                RoleMenuPermissions::Id,
                RoleMenuPermissions::RoleId,
                RoleMenuPermissions::MenuKey,
                RoleMenuPermissions::CreatedAt,
            ])
            .values_panic([
                binding_id.clone().into(),
                role_id.into(),
                menu_key.into(),
                now_rfc3339().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "role_menu_permissions", &binding_id, insert).await?;
    }
    Ok(())
}

async fn seed_role_ui_action_permissions<I, S>(
    manager: &SchemaManager<'_>,
    role_id: &str,
    ui_action_keys: I,
) -> Result<(), DbErr>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for ui_action_key in ui_action_keys {
        let ui_action_key = ui_action_key.as_ref();
        let binding_id = format!("rua-{role_id}-{ui_action_key}").replace(['/', '.'], "_");
        let insert = sea_query::Query::insert()
            .into_table(RoleUiActionPermissions::Table)
            .columns([
                RoleUiActionPermissions::Id,
                RoleUiActionPermissions::RoleId,
                RoleUiActionPermissions::UiActionKey,
                RoleUiActionPermissions::CreatedAt,
            ])
            .values_panic([
                binding_id.clone().into(),
                role_id.into(),
                ui_action_key.into(),
                now_rfc3339().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "role_ui_action_permissions", &binding_id, insert)
            .await?;
    }
    Ok(())
}

async fn sqlite_table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
        ))
        .await?;
    Ok(row.is_some())
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

async fn exec_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_database_backend(),
            sql.to_owned(),
        ))
        .await?;
    Ok(())
}

const ROLE_MANAGEMENT_PERMISSIONS: &[(&str, &str, &str, &str)] = &[
    (
        "perm-roles-read",
        "roles",
        "read",
        "Read roles and permission catalogs",
    ),
    (
        "perm-roles-manage",
        "roles",
        "manage",
        "Manage roles and permission matrices",
    ),
    (
        "perm-roles-assign",
        "roles",
        "assign",
        "Assign roles to users",
    ),
];

const ALL_MENU_KEYS: &[&str] = &[
    "/dashboard",
    "/jobs",
    "/workflows",
    "/instances",
    "/workers",
    "/workers/dispatch-queue",
    "/scripts",
    "/security",
    "/plugins",
    "/scopes",
    "/users",
    "/roles",
    "/calendars",
    "/api-keys",
    "/gitops",
    "/security",
    "/alerts",
    "/audit",
];
const OPERATOR_MENU_KEYS: &[&str] = &[
    "/dashboard",
    "/jobs",
    "/workflows",
    "/instances",
    "/workers",
    "/workers/dispatch-queue",
    "/scripts",
    "/security",
];
const VIEWER_MENU_KEYS: &[&str] = &[
    "/dashboard",
    "/jobs",
    "/workflows",
    "/instances",
    "/workers",
    "/scripts",
    "/security",
];

const ALL_UI_ACTION_KEYS: &[&str] = &[
    "users.create",
    "users.edit",
    "users.delete",
    "roles.create",
    "roles.edit",
    "roles.delete",
    "roles.permissions.edit",
    "jobs.create",
    "jobs.edit",
    "jobs.delete",
    "jobs.trigger",
    "scripts.create",
    "scripts.edit",
    "scripts.delete",
    "scripts.publish",
    "workflows.create",
    "workflows.edit",
    "workflows.run",
    "apiKeys.create",
    "apiKeys.edit",
    "apiKeys.delete",
];
const OPERATOR_UI_ACTION_KEYS: &[&str] =
    &["jobs.create", "jobs.edit", "jobs.trigger", "workflows.run"];
const VIEWER_UI_ACTION_KEYS: &[&str] = &[];

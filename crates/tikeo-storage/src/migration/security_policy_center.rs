use sea_orm_migration::prelude::{
    DbErr, MigrationName, MigrationTrait, SchemaManager, async_trait, sea_query,
};

use super::{
    Permissions, RoleMenuPermissions, exec_seed_insert_if_missing, now_rfc3339,
    seed_role_permissions,
};

pub(super) struct SecurityPolicyCenterMigration;

impl MigrationName for SecurityPolicyCenterMigration {
    fn name(&self) -> &'static str {
        "m20260617_000001_security_policy_center_rbac"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for SecurityPolicyCenterMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        seed_security_permissions(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn seed_security_permissions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now = now_rfc3339();
    for (id, resource, action, description) in [
        (
            "perm-security-read",
            "security",
            "read",
            "Read Security Policy Center posture and policy evidence",
        ),
        (
            "perm-security-manage",
            "security",
            "manage",
            "Manage Security Policy Center policies and enforcement settings",
        ),
    ] {
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
                id.into(),
                resource.into(),
                action.into(),
                description.into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "permissions", id, insert).await?;
    }
    seed_role_permissions(
        manager,
        "role-owner",
        ["perm-security-read", "perm-security-manage"],
    )
    .await?;
    seed_role_permissions(manager, "role-operator", ["perm-security-read"]).await?;
    seed_role_permissions(manager, "role-viewer", ["perm-security-read"]).await?;
    for role_id in ["role-owner", "role-operator", "role-viewer"] {
        let binding_id = format!("rmp-{role_id}-_security");
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
                "/security".into(),
                now.clone().into(),
            ])
            .to_owned();
        exec_seed_insert_if_missing(manager, "role_menu_permissions", &binding_id, insert).await?;
    }
    Ok(())
}

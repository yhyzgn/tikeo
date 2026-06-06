use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::{app, job, namespace, worker_pool};

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct NamespaceSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct AppSummary {
    pub id: String,
    pub namespace: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct WorkerPoolSummary {
    pub id: String,
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub max_queue_depth: i32,
    pub max_concurrency: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UpdateWorkerPoolQuota {
    pub max_queue_depth: i32,
    pub max_concurrency: i32,
}

#[derive(Debug, Clone)]
pub struct ScopeRepository {
    db: DatabaseConnection,
}

impl ScopeRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_namespaces(&self) -> Result<Vec<NamespaceSummary>, sea_orm::DbErr> {
        let rows = namespace::Entity::find().all(&self.db).await?;
        Ok(rows.into_iter().map(NamespaceSummary::from).collect())
    }

    pub async fn create_namespace(&self, name: &str) -> Result<NamespaceSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        Ok(NamespaceSummary::from(
            self.ensure_namespace(name, &now).await?,
        ))
    }

    pub async fn list_apps(
        &self,
        namespace_name: Option<&str>,
    ) -> Result<Vec<AppSummary>, sea_orm::DbErr> {
        let rows = app::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(ns) = namespace::Entity::find_by_id(row.namespace_id.clone())
                .one(&self.db)
                .await?
            else {
                continue;
            };
            if namespace_name.is_some_and(|wanted| wanted != ns.name) {
                continue;
            }
            out.push(AppSummary {
                id: row.id,
                namespace: ns.name,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
        Ok(out)
    }

    pub async fn create_app(
        &self,
        namespace_name: &str,
        name: &str,
    ) -> Result<AppSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let ns = self.ensure_namespace(namespace_name, &now).await?;
        let app = self.ensure_app(&ns.id, name, &now).await?;
        Ok(AppSummary {
            id: app.id,
            namespace: ns.name,
            name: app.name,
            created_at: app.created_at,
            updated_at: app.updated_at,
        })
    }

    pub async fn list_worker_pools(
        &self,
        namespace_name: Option<&str>,
        app_name: Option<&str>,
    ) -> Result<Vec<WorkerPoolSummary>, sea_orm::DbErr> {
        let rows = worker_pool::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(ns) = namespace::Entity::find_by_id(row.namespace_id.clone())
                .one(&self.db)
                .await?
            else {
                continue;
            };
            let Some(app) = app::Entity::find_by_id(row.app_id.clone())
                .one(&self.db)
                .await?
            else {
                continue;
            };
            if namespace_name.is_some_and(|wanted| wanted != ns.name)
                || app_name.is_some_and(|wanted| wanted != app.name)
            {
                continue;
            }
            out.push(WorkerPoolSummary {
                id: row.id,
                namespace: ns.name,
                app: app.name,
                name: row.name,
                max_queue_depth: row.max_queue_depth,
                max_concurrency: row.max_concurrency,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
        Ok(out)
    }

    pub async fn create_worker_pool(
        &self,
        namespace_name: &str,
        app_name: &str,
        name: &str,
    ) -> Result<WorkerPoolSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let ns = self.ensure_namespace(namespace_name, &now).await?;
        let app = self.ensure_app(&ns.id, app_name, &now).await?;
        if let Some(existing) = worker_pool::Entity::find()
            .filter(worker_pool::Column::AppId.eq(app.id.clone()))
            .filter(worker_pool::Column::Name.eq(name))
            .one(&self.db)
            .await?
        {
            return Ok(WorkerPoolSummary {
                id: existing.id,
                namespace: ns.name,
                app: app.name,
                name: existing.name,
                max_queue_depth: existing.max_queue_depth,
                max_concurrency: existing.max_concurrency,
                created_at: existing.created_at,
                updated_at: existing.updated_at,
            });
        }
        let model = worker_pool::ActiveModel {
            id: Set(new_id("wp")),
            namespace_id: Set(ns.id.clone()),
            app_id: Set(app.id.clone()),
            name: Set(name.to_owned()),
            max_queue_depth: Set(0),
            max_concurrency: Set(0),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(WorkerPoolSummary {
            id: model.id,
            namespace: ns.name,
            app: app.name,
            name: model.name,
            max_queue_depth: model.max_queue_depth,
            max_concurrency: model.max_concurrency,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }

    pub async fn update_worker_pool_quota(
        &self,
        id: &str,
        input: UpdateWorkerPoolQuota,
    ) -> Result<Option<WorkerPoolSummary>, sea_orm::DbErr> {
        let Some(existing) = worker_pool::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(ns) = namespace::Entity::find_by_id(existing.namespace_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(app) = app::Entity::find_by_id(existing.app_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: worker_pool::ActiveModel = existing.into();
        active.max_queue_depth = Set(input.max_queue_depth.max(0));
        active.max_concurrency = Set(input.max_concurrency.max(0));
        active.updated_at = Set(now_rfc3339());
        let model = active.update(&self.db).await?;
        Ok(Some(WorkerPoolSummary {
            id: model.id,
            namespace: ns.name,
            app: app.name,
            name: model.name,
            max_queue_depth: model.max_queue_depth,
            max_concurrency: model.max_concurrency,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }))
    }

    pub async fn delete_namespace_if_empty(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(ns) = namespace::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let app_count = app::Entity::find()
            .filter(app::Column::NamespaceId.eq(ns.id.clone()))
            .all(&self.db)
            .await?
            .len();
        let pool_count = worker_pool::Entity::find()
            .filter(worker_pool::Column::NamespaceId.eq(ns.id.clone()))
            .all(&self.db)
            .await?
            .len();
        let job_count = job::Entity::find()
            .filter(job::Column::NamespaceId.eq(ns.id.clone()))
            .all(&self.db)
            .await?
            .len();
        if app_count + pool_count + job_count > 0 {
            return Err(sea_orm::DbErr::Custom("namespace is not empty".to_owned()));
        }
        namespace::Entity::delete_by_id(ns.id)
            .exec(&self.db)
            .await?;
        Ok(true)
    }

    pub async fn delete_app_if_empty(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(app_model) = app::Entity::find_by_id(id.to_owned()).one(&self.db).await? else {
            return Ok(false);
        };
        let pool_count = worker_pool::Entity::find()
            .filter(worker_pool::Column::AppId.eq(app_model.id.clone()))
            .all(&self.db)
            .await?
            .len();
        let job_count = job::Entity::find()
            .filter(job::Column::AppId.eq(app_model.id.clone()))
            .all(&self.db)
            .await?
            .len();
        if pool_count + job_count > 0 {
            return Err(sea_orm::DbErr::Custom("app is not empty".to_owned()));
        }
        app::Entity::delete_by_id(app_model.id)
            .exec(&self.db)
            .await?;
        Ok(true)
    }

    pub async fn delete_worker_pool(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = worker_pool::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    async fn ensure_namespace(
        &self,
        name: &str,
        now: &str,
    ) -> Result<namespace::Model, sea_orm::DbErr> {
        if let Some(model) = namespace::Entity::find()
            .filter(namespace::Column::Name.eq(name))
            .one(&self.db)
            .await?
        {
            return Ok(model);
        }
        namespace::ActiveModel {
            id: Set(new_id("ns")),
            name: Set(name.to_owned()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await
    }

    async fn ensure_app(
        &self,
        namespace_id: &str,
        name: &str,
        now: &str,
    ) -> Result<app::Model, sea_orm::DbErr> {
        if let Some(model) = app::Entity::find()
            .filter(app::Column::NamespaceId.eq(namespace_id))
            .filter(app::Column::Name.eq(name))
            .one(&self.db)
            .await?
        {
            return Ok(model);
        }
        app::ActiveModel {
            id: Set(new_id("app")),
            namespace_id: Set(namespace_id.to_owned()),
            name: Set(name.to_owned()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await
    }
}

impl From<namespace::Model> for NamespaceSummary {
    fn from(value: namespace::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

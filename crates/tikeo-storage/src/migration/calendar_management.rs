use super::{string_col, string_pk, text_col};
use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum Calendars {
    Table,
    Id,
    Namespace,
    App,
    Name,
    Timezone,
    ExcludedDatesJson,
    HolidaysJson,
    MaintenanceWindowsJson,
    FreezeWindowsJson,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

pub(super) struct CalendarManagementMigration;

impl MigrationName for CalendarManagementMigration {
    fn name(&self) -> &'static str {
        "m20260630_000001_calendar_management"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CalendarManagementMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_calendars(manager).await?;
        create_calendar_indexes(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn create_calendars(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Calendars::Table)
                .if_not_exists()
                .col(string_pk(Calendars::Id))
                .col(string_col(Calendars::Namespace))
                .col(string_col(Calendars::App))
                .col(string_col(Calendars::Name))
                .col(string_col(Calendars::Timezone))
                .col(text_col(Calendars::ExcludedDatesJson))
                .col(text_col(Calendars::HolidaysJson))
                .col(text_col(Calendars::MaintenanceWindowsJson))
                .col(text_col(Calendars::FreezeWindowsJson))
                .col(string_col(Calendars::CreatedBy))
                .col(string_col(Calendars::CreatedAt))
                .col(string_col(Calendars::UpdatedAt))
                .to_owned(),
        )
        .await
}

async fn create_calendar_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("idx_calendars_scope_name")
                .table(Calendars::Table)
                .col(Calendars::Namespace)
                .col(Calendars::App)
                .col(Calendars::Name)
                .unique()
                .if_not_exists()
                .to_owned(),
        )
        .await
}

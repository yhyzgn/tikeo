use sea_orm_migration::prelude::*;

pub(super) fn string_pk<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column)
        .string_len(191)
        .not_null()
        .primary_key()
        .take()
}

pub(super) fn string_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(191).not_null().take()
}

pub(super) fn string_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(191).null().take()
}

pub(super) fn short_string_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(128).not_null().take()
}

pub(super) fn text_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).text().not_null().take()
}

pub(super) fn text_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).text().null().take()
}

pub(super) fn boolean_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).boolean().not_null().take()
}

pub(super) fn big_integer_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).big_integer().not_null().take()
}

pub(super) fn big_integer_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).big_integer().null().take()
}

pub(super) fn integer_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).integer().not_null().take()
}

pub(super) fn integer_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).integer().null().take()
}

use sea_orm_migration::prelude::*;

/// String pk.
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

/// String col.
pub(super) fn string_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(191).not_null().take()
}

/// String null.
pub(super) fn string_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(191).null().take()
}

/// Short string col.
pub(super) fn short_string_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).string_len(128).not_null().take()
}

/// Text col.
pub(super) fn text_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).text().not_null().take()
}

/// Text null.
pub(super) fn text_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).text().null().take()
}

/// Boolean col.
pub(super) fn boolean_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).boolean().not_null().take()
}

/// Big integer col.
pub(super) fn big_integer_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).big_integer().not_null().take()
}

/// Big integer null.
pub(super) fn big_integer_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).big_integer().null().take()
}

/// Integer col.
pub(super) fn integer_col<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).integer().not_null().take()
}

/// Integer null.
pub(super) fn integer_null<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column).integer().null().take()
}

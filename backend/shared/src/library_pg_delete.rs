use uuid::Uuid;

use crate::auth::UserContext;
use crate::error::AppResult;

use super::{
    database_error,
    library_pg_sql::{DELETE_ITEM, RECORD_ITEM_DELETION},
    not_found, PgLibraryService,
};

pub(super) async fn delete_item(
    service: &PgLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<()> {
    let user_id = service.required_user_id(user, item_id).await?;
    let mut transaction = service.db.begin().await.map_err(database_error)?;
    sqlx::query(RECORD_ITEM_DELETION)
        .bind(user_id)
        .bind(item_id)
        .execute(&mut *transaction)
        .await
        .map_err(database_error)?;
    let result = sqlx::query(DELETE_ITEM)
        .bind(item_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(database_error)?;
    if result.rows_affected() == 0 {
        return Err(not_found(item_id));
    }
    transaction.commit().await.map_err(database_error)?;
    Ok(())
}

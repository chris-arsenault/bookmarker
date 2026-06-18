use uuid::Uuid;

use crate::auth::UserContext;
use crate::error::AppResult;

use super::{database_error, library_pg_sql::DELETE_ITEM, not_found, PgLibraryService};

pub(super) async fn delete_item(
    service: &PgLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<()> {
    let user_id = service.required_user_id(user, item_id).await?;
    let result = sqlx::query(DELETE_ITEM)
        .bind(item_id)
        .bind(user_id)
        .execute(&service.db)
        .await
        .map_err(database_error)?;
    if result.rows_affected() == 0 {
        return Err(not_found(item_id));
    }
    Ok(())
}

use crate::{
    app_error::DbError,
    db::{
        admin_op_log_rows::AdminOpLogRow, admin_user_rows::AdminUserRow,
        cli_session_rows::CliSessionRow, pool::DbPool,
    },
};

pub struct AdminRepo {
    pub pool: DbPool,
}

impl AdminRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AdminUserRow>, DbError> {
        let row = sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT user_id, username, password_hash, is_active, created_at, updated_at
            FROM admin_users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row)
    }

    pub async fn find_user_by_id(&self, user_id: i64) -> Result<Option<AdminUserRow>, DbError> {
        let row = sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT user_id, username, password_hash, is_active, created_at, updated_at
            FROM admin_users
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row)
    }

    pub async fn insert_session(&self, session: &CliSessionRow) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO cli_sessions (user_id, session_token, issued_at, expires_at, revoked_at)
            VALUES ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(session.user_id)
        .bind(&session.session_token)
        .bind(&session.issued_at)
        .bind(&session.expires_at)
        .bind(&session.revoked_at)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }

    pub async fn find_session_by_token(
        &self,
        token: &str,
    ) -> Result<Option<CliSessionRow>, DbError> {
        let row = sqlx::query_as::<_, CliSessionRow>(
            r#"
            SELECT session_id, user_id, session_token, issued_at, expires_at, revoked_at
            FROM cli_sessions
            WHERE session_token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row)
    }

    pub async fn insert_op_log(&self, log: &AdminOpLogRow) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO admin_op_logs (user_id, op_type, app_uuid, op_params, created_at)
            VALUES ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(log.user_id)
        .bind(&log.op_type)
        .bind(log.app_uuid)
        .bind(&log.op_params)
        .bind(&log.created_at)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }
}

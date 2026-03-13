use crate::{
    app_error::DbError,
    db::{app_rows::AppRow, pool::DbPool},
};
use uuid::Uuid;

pub struct AppsRepo {
    pub pool: DbPool,
}

impl AppsRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn fetch_all(&self) -> Result<Vec<AppRow>, DbError> {
        let rows = sqlx::query_as::<_, AppRow>(
            r#"
            SELECT
              app_uuid, name, target_url, status,
              mount_path, upstream_path,
              app_secret,
              rate_limit_rps, allowed_source_ips, blocked_source_ips,
              created_at, updated_at
            FROM apps
            ORDER BY name
            "#,
        )
        .fetch_all(self.pool.inner())
        .await?;

        Ok(rows)
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<AppRow>, DbError> {
        let row = sqlx::query_as::<_, AppRow>(
            r#"
            SELECT
              app_uuid, name, target_url, status,
              mount_path, upstream_path,
              app_secret,
              rate_limit_rps, allowed_source_ips, blocked_source_ips,
              created_at, updated_at
            FROM apps
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row)
    }

    pub async fn find_by_uuid(&self, app_uuid: Uuid) -> Result<Option<AppRow>, DbError> {
        let row = sqlx::query_as::<_, AppRow>(
            r#"
            SELECT
              app_uuid, name, target_url, status,
              mount_path, upstream_path,
              app_secret,
              rate_limit_rps, allowed_source_ips, blocked_source_ips,
              created_at, updated_at
            FROM apps
            WHERE app_uuid = $1
            "#,
        )
        .bind(app_uuid)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row)
    }

    pub async fn insert(&self, app: &AppRow) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO apps (
              app_uuid, name, target_url, status,
              mount_path, upstream_path,
              app_secret,
              rate_limit_rps, allowed_source_ips, blocked_source_ips,
              created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(app.app_uuid)
        .bind(&app.name)
        .bind(&app.target_url)
        .bind(&app.status)
        .bind(&app.mount_path)
        .bind(&app.upstream_path)
        .bind(&app.app_secret)
        .bind(app.rate_limit_rps)
        .bind(&app.allowed_source_ips)
        .bind(&app.blocked_source_ips)
        .bind(&app.created_at)
        .bind(&app.updated_at)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }

    pub async fn update_route(
        &self,
        name: &str,
        mount_path: &str,
        upstream_path: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE apps
            SET mount_path = $2, upstream_path = $3, updated_at = now()
            WHERE name = $1
            "#,
        )
        .bind(name)
        .bind(mount_path)
        .bind(upstream_path)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }

    pub async fn update_status(&self, name: &str, status: &str) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE apps
            SET status = $2, updated_at = now()
            WHERE name = $1
            "#,
        )
        .bind(name)
        .bind(status)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }
}

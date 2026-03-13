use crate::{
    app_error::DbError,
    db::{health_rows::AppHealthRow, pool::DbPool},
};
use uuid::Uuid;

pub struct HealthRepo {
    pub pool: DbPool,
}

impl HealthRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_latest(&self, row: &AppHealthRow) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO app_health_latest (
              app_uuid, last_checked_at, ok, status_code, latency_ms, updated_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            ON CONFLICT (app_uuid) DO UPDATE SET
              last_checked_at = EXCLUDED.last_checked_at,
              ok = EXCLUDED.ok,
              status_code = EXCLUDED.status_code,
              latency_ms = EXCLUDED.latency_ms,
              updated_at = now()
            "#,
        )
        .bind(row.app_uuid)
        .bind(row.last_checked_at)
        .bind(row.ok)
        .bind(row.status_code)
        .bind(row.latency_ms)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    pub async fn upsert_many_latest(&self, rows: &[AppHealthRow]) -> Result<(), DbError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.inner().begin().await?;
        for row in rows {
            sqlx::query(
                r#"
                INSERT INTO app_health_latest (
                  app_uuid, last_checked_at, ok, status_code, latency_ms, updated_at
                ) VALUES ($1,$2,$3,$4,$5, now())
                ON CONFLICT (app_uuid) DO UPDATE SET
                  last_checked_at = EXCLUDED.last_checked_at,
                  ok = EXCLUDED.ok,
                  status_code = EXCLUDED.status_code,
                  latency_ms = EXCLUDED.latency_ms,
                  updated_at = now()
                "#,
            )
            .bind(row.app_uuid)
            .bind(row.last_checked_at)
            .bind(row.ok)
            .bind(row.status_code)
            .bind(row.latency_ms)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn fetch_all_latest(&self) -> Result<Vec<AppHealthRow>, DbError> {
        let rows = sqlx::query_as::<_, AppHealthRow>(
            r#"
            SELECT app_uuid, last_checked_at, ok, status_code, latency_ms
            FROM app_health_latest
            ORDER BY app_uuid
            "#,
        )
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows)
    }

    pub async fn find_latest_by_uuid(&self, app_uuid: Uuid) -> Result<Option<AppHealthRow>, DbError> {
        let row = sqlx::query_as::<_, AppHealthRow>(
            r#"
            SELECT app_uuid, last_checked_at, ok, status_code, latency_ms
            FROM app_health_latest
            WHERE app_uuid = $1
            "#,
        )
        .bind(app_uuid)
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(row)
    }
}

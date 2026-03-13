use std::time::Duration;

use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

use crate::app_error::DbError;

/// Thin newtype wrapper so the rest of the codebase does not depend on
/// sqlx concrete types directly.
#[derive(Clone)]
pub struct DbPool(pub Pool<Postgres>);

impl DbPool {
    /// Create a new Postgres pool using the provided settings.
    pub async fn connect(
        database_url: &str,
        max_connections: u32,
        connect_timeout: Duration,
    ) -> Result<Self, DbError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(connect_timeout)
            .connect(database_url)
            .await?;

        Ok(Self(pool))
    }

    /// Access the inner sqlx pool when needed.
    pub fn inner(&self) -> &Pool<Postgres> {
        &self.0
    }
}

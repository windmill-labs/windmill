//! Database connection and initialization for local mode

use libsql::{Builder, Connection, Database};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::schema;

/// Local database wrapper
///
/// For local mode, we use a single connection with in-process coordination.
/// This simplifies the implementation since we don't need `FOR UPDATE SKIP LOCKED`.
pub struct LocalDb {
    #[allow(dead_code)]
    db: Database,
    /// Single connection for all operations (simplifies transaction handling)
    conn: Arc<Mutex<Connection>>,
}

impl LocalDb {
    /// Create an in-memory database (for testing/ephemeral use)
    pub async fn in_memory() -> Result<Self> {
        let db = Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;
        let local_db = Self {
            db,
            conn: Arc::new(Mutex::new(conn)),
        };
        local_db.init_schema().await?;
        Ok(local_db)
    }

    /// Create a file-based database
    pub async fn file(path: &str) -> Result<Self> {
        let db = Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        let local_db = Self {
            db,
            conn: Arc::new(Mutex::new(conn)),
        };
        local_db.init_schema().await?;
        Ok(local_db)
    }

    /// Create a Turso remote database connection
    /// This would be used for the multi-writer scenario
    pub async fn turso_remote(url: &str, auth_token: &str) -> Result<Self> {
        let db = Builder::new_remote(url.to_string(), auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        let local_db = Self {
            db,
            conn: Arc::new(Mutex::new(conn)),
        };
        local_db.init_schema().await?;
        Ok(local_db)
    }

    /// Initialize the schema
    async fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        // Execute schema as multiple statements
        conn.execute_batch(schema::SCHEMA).await?;
        Ok(())
    }

    /// Reset the database (drop and recreate all tables)
    pub async fn reset(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(schema::DROP_SCHEMA).await?;
        conn.execute_batch(schema::SCHEMA).await?;
        Ok(())
    }

    /// Get a reference to the connection (locked)
    pub async fn conn(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    /// Execute a simple query that returns no rows
    pub async fn execute(&self, sql: &str, params: impl libsql::params::IntoParams) -> Result<u64> {
        let conn = self.conn.lock().await;
        let rows_affected = conn.execute(sql, params).await?;
        Ok(rows_affected)
    }

    /// Execute a query and return all rows
    pub async fn query(
        &self,
        sql: &str,
        params: impl libsql::params::IntoParams,
    ) -> Result<libsql::Rows> {
        let conn = self.conn.lock().await;
        let rows = conn.query(sql, params).await?;
        Ok(rows)
    }

    /// Execute a batch of statements (for transactions)
    pub async fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(sql).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_db() {
        let db = LocalDb::in_memory().await.unwrap();

        // Verify tables exist
        let rows = db
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                (),
            )
            .await
            .unwrap();

        let mut tables = Vec::new();
        let mut rows = rows;
        while let Some(row) = rows.next().await.unwrap() {
            let name: String = row.get(0).unwrap();
            tables.push(name);
        }

        assert!(tables.contains(&"v2_job".to_string()));
        assert!(tables.contains(&"v2_job_queue".to_string()));
        assert!(tables.contains(&"v2_job_completed".to_string()));
    }

    #[tokio::test]
    async fn test_reset_db() {
        let db = LocalDb::in_memory().await.unwrap();

        // Insert a job
        db.execute(
            "INSERT INTO v2_job (id, kind) VALUES ('test-uuid', 'preview')",
            (),
        )
        .await
        .unwrap();

        // Reset
        db.reset().await.unwrap();

        // Verify job is gone
        let mut rows = db
            .query("SELECT COUNT(*) FROM v2_job", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let count: i64 = row.get(0).unwrap();
        assert_eq!(count, 0);
    }
}

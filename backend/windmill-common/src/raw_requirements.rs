use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

use crate::{error, scripts::ScriptLang};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct RawRequirements {
    pub id: i64,
    pub workspace_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub language: ScriptLang,
    pub path: String,
    pub archived: bool,
    pub content: String,
}

#[derive(sqlx::FromRow, Clone, Serialize, Deserialize, Hash, Debug)]
pub struct ViewableRawRequirements {
    pub workspace_id: String,
    pub language: ScriptLang,
    pub path: String,
    pub content: String,
}

impl ViewableRawRequirements {
    // TODO(claude): add docs
    pub async fn create<'c>(self, e: impl PgExecutor<'c>) -> error::Result<i64> {
        sqlx::query_scalar!(
            "
            WITH _ AS (
                UPDATE raw_requirements 
                SET archived = true 
                WHERE archived = false
                    AND path = $1
                    AND workspace_id = $2
                    AND language = $4
            )
            INSERT INTO raw_requirements (path, workspace_id, content, language)
            VALUES ($1, $2, $3, $4) 
            RETURNING id
            ",
            self.path,
            self.workspace_id,
            self.content,
            self.language as ScriptLang
        )
        .fetch_one(e)
        .await
        .map_err(error::Error::from)
    }

    // TODO(claude): add docs
    pub async fn list<'c>(workspace_id: &str, e: impl PgExecutor<'c>) -> error::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r##"
            SELECT path, workspace_id, content, language AS "language: ScriptLang"
                FROM raw_requirements
                WHERE archived = false AND workspace_id = $1
            "##,
            workspace_id,
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }
}

impl RawRequirements {
    /// Archives raw requirements by marking them as archived for the given path and workspace.
    ///
    /// This sets the `archived` flag to `true` for all non-archived raw requirements
    /// matching the specified path and workspace ID.
    pub async fn archive<'c>(
        path: &str,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        sqlx::query!(
            "
            UPDATE raw_requirements
            SET archived = true
            WHERE path = $1 AND workspace_id = $2 AND archived = false
            ",
            path,
            workspace_id,
        )
        .execute(e)
        .await?;
        Ok(())
    }

    /// Permanently deletes raw requirements from the database.
    ///
    /// This removes all raw requirements records matching the specified path and workspace ID.
    /// Use with caution as this operation cannot be undone.
    pub async fn delete<'c>(
        path: &str,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        sqlx::query!(
            "
            DELETE
            FROM raw_requirements
            WHERE path = $1 AND workspace_id = $2
            ",
            path,
            workspace_id
        )
        .execute(e)
        .await?;
        Ok(())
    }

    /// Retrieves the latest raw requirements for a runnable path.
    ///
    /// This method finds requirements where the requirements path is a prefix of the runnable path.
    /// For example, if runnable_path is "folder/script" and there are requirements at "folder/",
    /// it will return those requirements. Only non-archived requirements are considered.
    ///
    /// Returns the first matching requirement or None if no matches are found.
    pub async fn get_latest_for_runnable<'c>(
        // If runnable path is LIKE path of requirements, this requirement will be returened.
        // You can also set path of requirements and then it will just query them
        runnable_path: &str,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<RawRequirements>> {
        sqlx::query_as!(
            RawRequirements,
            r#"
            SELECT id, content, language AS "language: ScriptLang", path, archived, workspace_id, created_at
            FROM raw_requirements
            WHERE path LIKE $1 || '%' -- path should start with runnable_path
                AND workspace_id = $2
                AND archived = false
            LIMIT 1
            "#,
            runnable_path,
            workspace_id
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)
    }

    /// Retrieves a specific raw requirements record by ID and workspace.
    ///
    /// Returns the raw requirements record matching the provided ID and workspace ID,
    /// or None if no such record exists.
    pub async fn get<'c>(
        id: i64,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<RawRequirements>> {
        sqlx::query_as!(
            RawRequirements,
            r#"
            SELECT id, content, language AS "language: ScriptLang", path, archived, workspace_id, created_at
            FROM raw_requirements
            WHERE id = $1 AND workspace_id = $2
            LIMIT 1
            "#,
            id,
            workspace_id
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)
    }

    /// Retrieves all version IDs for raw requirements at a specific path.
    ///
    /// Returns a vector of IDs for all raw requirements records (including archived)
    /// matching the specified path and workspace ID, ordered by creation time.
    pub async fn get_history<'c>(
        path: &str,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Vec<i64>> {
        sqlx::query_scalar!(
            r#"
            SELECT id FROM raw_requirements
            WHERE path = $1 AND workspace_id = $2
            "#,
            path,
            workspace_id
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }
}

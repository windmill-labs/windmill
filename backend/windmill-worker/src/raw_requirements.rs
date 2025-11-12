use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use windmill_common::{error, scripts::ScriptLang};

use crate::trigger_dependents_to_recompute_dependencies;

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct RawRequirements {
    pub id: i64,
    pub name: Option<String>,
    pub workspace_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub language: ScriptLang,
    pub archived: bool,
    pub content: String,
}

#[derive(sqlx::FromRow, Clone, Serialize, Deserialize, Hash, Debug)]
pub struct NewRawRequirements {
    pub workspace_id: String,
    pub language: ScriptLang,
    pub name: Option<String>,
    // TODO: Make Option, or optimize it in any other way.
    pub content: String,
}

impl NewRawRequirements {
    // TODO(claude): add docs
    pub async fn create<'c>(self, db: &sqlx::Pool<sqlx::Postgres>) -> error::Result<i64> {
        // Enforce validation early.
        let path = RawRequirements::to_path(&self.name, self.language)?;
        let new_id = sqlx::query_scalar!(
            "
            WITH _ AS (
                UPDATE raw_requirements 
                SET archived = true 
                WHERE archived = false
                    AND name = $1
                    AND workspace_id = $2
                    AND language = $4
            )
            INSERT INTO raw_requirements (name, workspace_id, content, language)
            VALUES ($1, $2, $3, $4) 
            RETURNING id
            ",
            self.name,
            self.workspace_id,
            self.content,
            self.language as ScriptLang
        )
        .fetch_one(db)
        .await?;

        trigger_dependents_to_recompute_dependencies(
            &self.workspace_id,
            path.as_str(),
            None, // TODO
            None, // TODO
            "",   // TODO
            "",   // TODO
            "",   // TODO
            db,   // TODO
            vec![],
        )
        .await?;

        Ok(new_id)
    }
}

impl RawRequirements {
    pub fn to_path(name: &Option<String>, language: ScriptLang) -> error::Result<String> {
        let requirements_filename =
            language
                .as_requirements_filename()
                .ok_or(error::Error::BadConfig(format!(
                    "raw requirements are not supported for: {}",
                    language.as_str()
                )))?;

        Ok(if let Some(name) = name {
            format!("raw_requirements/{name}.{requirements_filename}")
        } else {
            format!("raw_requirements/{requirements_filename}")
        })
    }
    // TODO(claude): add docs
    pub async fn archive<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        sqlx::query!(
            "
            UPDATE raw_requirements
            SET archived = true
            WHERE name = $1 AND workspace_id = $2 AND archived = false AND language = $3
            ",
            name,
            workspace_id,
            language as ScriptLang
        )
        .execute(e)
        .await?;
        Ok(())
    }

    // TODO(claude): add docs
    pub async fn delete<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        sqlx::query!(
            "
            DELETE
            FROM raw_requirements
            WHERE name = $1 AND workspace_id = $2 AND language = $3
            ",
            name,
            workspace_id,
            language as ScriptLang
        )
        .execute(e)
        .await?;
        Ok(())
    }

    // TODO(claude): add docs
    pub async fn list<'c>(workspace_id: &str, e: impl PgExecutor<'c>) -> error::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r##"
            SELECT id, created_at, archived, name, workspace_id, content, language AS "language: ScriptLang"
                FROM raw_requirements
                WHERE archived = false AND workspace_id = $1
            "##,
            workspace_id,
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }

    // TODO(claude): add docs
    pub async fn get_latest<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<RawRequirements>> {
        if name == Some("none".to_owned()) {
            return Ok(None);
        }

        sqlx::query_as!(
            RawRequirements,
            r#"
            SELECT id, content, language AS "language: ScriptLang", name, archived, workspace_id, created_at
            FROM raw_requirements
            WHERE name = $1 AND workspace_id = $2 AND archived = false AND language = $3
            LIMIT 1
            "#,
            name,
            workspace_id,
            language as ScriptLang
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)
    }

    // TODO(claude): add docs
    pub async fn get<'c>(
        id: i64,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<RawRequirements>> {
        sqlx::query_as!(
            RawRequirements,
            r#"
            SELECT id, content, language AS "language: ScriptLang", name, archived, workspace_id, created_at
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

    // TODO(claude): add docs
    pub async fn get_history<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Vec<i64>> {
        sqlx::query_scalar!(
            r#"
            SELECT id FROM raw_requirements
            WHERE name = $1 AND workspace_id = $2 AND language = $3
            "#,
            name,
            workspace_id,
            language as ScriptLang
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }
}

// TODO:
// -[] Fork workspaces
// -[] git sync
// -[] handle renames
// -[] rebuild dependency map correctly
// -[] deployment of many at the same time has proper ordering
// -[] cli on generate-metadata will only send diffs
// -[] raw requirements are on by default for flows (cli)
// -[] if default rrs it has no entries in dmap, but they are always used unless told otherwise
// -[] rrs is writable by everyone unless is used by priviledged runnable (editable by admin/hidden)
// -[] docs
// -[] do we need min_version?
// -[] delete should also trigger redeploy
//
// TODO(frontend):
// - warn on redeploy. (if change will affect runnables, it will warn that it will redeploy other scripts as well (which (show recursively)))
// - deployed runnable should show backlink to rrs.
// - warn on rename - renaming will not be reflected in existing scripts, so the linkage will break. (Show which runnables it references). It is subject to change.
// - deploy should scroll up to show the warning.
//
// TODO(tests):
// - old syntax rejection
// - dmap rebuild (with and without relative imports) (and for default rrs)
// - redeployment of raw reqs redeploy all dependents (recursively)
// - redelpoyment of dependents or new deployments build dmap (with and without relative imports)
// - messed up rrs table will get healed
// - race condition with other djobs on concurrency basis
// - redeployment of relative imports will not capture djob, but it will propagete recursively AND it will create new versions.
// - what if redeployed older script version that has either outdated syntax or other rrs id/name?
// - how are python version are treated? From lock or from content?
//
// cli:
// - deployment of many at the same time has proper ordering

#[cfg(test)]
mod raw_requirements_tests {

    mod new_raw_requirements {
        use windmill_common::scripts::ScriptLang;

        use crate::raw_requirements::NewRawRequirements;

        #[cfg(feature = "python")]
        #[sqlx::test(
            fixtures("../../tests/fixtures/base.sql",),
            migrations = "../migrations"
        )]
        async fn test_create(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            assert_eq!(
                NewRawRequirements {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    name: None,
                    content: "global:rev1".to_owned(),
                }
                .create(&db)
                .await
                .unwrap(),
                1
            );

            assert_eq!(
                NewRawRequirements {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    name: Some("rrs1".to_owned()),
                    content: "rrs1:rev1".to_owned(),
                }
                .create(&db)
                .await
                .unwrap(),
                2
            );

            assert!(NewRawRequirements {
                workspace_id: "test-workspace".into(),
                language: ScriptLang::DuckDb,
                name: None,
                content: "".to_owned(),
            }
            .create(&db)
            .await
            .is_err());

            // Will act as redeployment
            assert_eq!(
                NewRawRequirements {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    name: Some("rrs1".to_owned()),
                    content: "rrs1:rev2".to_owned(),
                }
                .create(&db)
                .await
                .unwrap(),
                // It will just increment id
                3
            );
            Ok(())
        }
    }

    mod raw_requirements {
        // #[sqlx::test(fixtures("../../migrations/20251106152104_raw_requirements.up.sql"))]
        // async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
        //     todo!()
        // }
    }
}

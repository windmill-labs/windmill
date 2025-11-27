use serde::{Deserialize, Serialize};
use windmill_common::{error, scripts::ScriptLang, workspace_dependencies::WorkspaceDependencies};

use crate::{
    scoped_dependency_map::ScopedDependencyMap, trigger_dependents_to_recompute_dependencies,
};

#[derive(sqlx::FromRow, Clone, Serialize, Deserialize, Hash, Debug)]
pub struct NewWorkspaceDependencies {
    pub workspace_id: String,
    pub language: ScriptLang,
    pub name: Option<String>,
    /// If None, will use description of previous version
    /// If there is no older versions, will set to default
    pub description: Option<String>,
    // TODO: Make Option, or optimize it in any other way.
    pub content: String,
}

impl NewWorkspaceDependencies {
    /// Creates a new workspace dependencies entry in the database.
    ///
    /// Archives any existing dependencies with the same name/language/workspace,
    /// then inserts the new dependencies. Triggers recomputation of dependent scripts
    /// and rebuilds the dependency map if this is the first unnamed dependency for the workspace.
    pub async fn create<'c>(
        self,
        email: &str,
        created_by: &str,
        permissioned_as: &str,
        db: &sqlx::Pool<sqlx::Postgres>,
    ) -> error::Result<i64> {
        // Check if all workers support workspace dependencies feature
        windmill_common::workspace_dependencies::min_version_supports_v0_workspace_dependencies()
            .await?;

        let path = WorkspaceDependencies::to_path(&self.name, self.language)?;

        // If it is unnamed then we want to rebuild dependency map. Otherwise trigger dependents to recompute locks will not work
        // NOTE: We rebuild first, even before creating new w deps. We want to make sure that if rebuild failed, then no new default workspace dependencies were created.
        if self.name.is_none() {
            // Check if we already rebuilt the map for this workspace by checking if the setting exists
            let setting_name = format!("workspace_dependencies_map_rebuilt:{}", self.workspace_id);
            let already_rebuilt =
                windmill_common::global_settings::load_value_from_global_settings(
                    db,
                    &setting_name,
                )
                .await?
                .is_some();

            if !already_rebuilt {
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Rebuilding workspace dependencies map for first unnamed workspace dependencies"
                );
                ScopedDependencyMap::rebuild_map_unchecked(&self.workspace_id, db).await?;

                // Mark as rebuilt by creating the setting
                windmill_common::global_settings::set_value_in_global_settings(
                    db,
                    &setting_name,
                    serde_json::json!({}),
                )
                .await?;
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Marked workspace dependencies map as rebuilt"
                );
            } else {
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Skipping workspace dependencies map rebuild - already rebuilt for this workspace"
                );
            }
        };

        let mut tx = db.begin().await?;
        let prev_description = sqlx::query_scalar!(
            "
                UPDATE workspace_dependencies
                SET archived = true 
                WHERE archived = false
                    AND name IS NOT DISTINCT FROM $1
                    AND workspace_id = $2
                    AND language = $3
                RETURNING description
            ",
            self.name,
            self.workspace_id,
            self.language as ScriptLang
        )
        .fetch_optional(&mut *tx)
        .await?;

        let new_id = sqlx::query_scalar!(
            "
            INSERT INTO workspace_dependencies(name, workspace_id, content, language, description)
            VALUES ($1, $2, $3, $4, $5) 
            RETURNING id
            ",
            self.name.clone(),
            self.workspace_id,
            self.content,
            self.language as ScriptLang,
            self.description
                .or(prev_description.clone())
                .unwrap_or("Default Workspace Dependencies".to_owned())
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        // Make sure trigger dependents will have latest view.
        // NOTE: Uncomment for tests
        // #[cfg(test)]
        // assert_eq!(
        //     sqlx::query_scalar!(
        //         "
        //         SELECT id FROM workspace_dependencies
        //         WHERE archived = false
        //             AND name IS NOT DISTINCT FROM $1
        //             AND workspace_id = $2
        //             AND language = $3
        //         ",
        //         self.name,
        //         self.workspace_id,
        //         self.language as ScriptLang,
        //     )
        //     .fetch_one(db) // Use db
        //     .await?,
        //     new_id
        // );

        // It's ok to fail, it will return an error and user will get notified that they should redeploy workspace dependencies
        trigger_dependents_to_recompute_dependencies(
            &self.workspace_id,
            crate::scoped_dependency_map::ScopedDependencyMap::get_dependents(
                path.as_str(),
                &self.workspace_id,
                db,
            )
            .await?,
            None,
            None,
            email,
            created_by,
            permissioned_as,
            db,
            vec![],
        )
        .await?;

        Ok(new_id)
    }
}

// Type aliases for backward compatibility
pub type RawRequirements = WorkspaceDependencies;
pub type NewRawRequirements = NewWorkspaceDependencies;

#[cfg(test)]
mod workspace_dependencies_tests {

    // TODO: test all cases when it should reject.
    #[cfg(feature = "python")]
    mod new_workspace_dependencies {
        use windmill_common::scripts::ScriptLang;

        use crate::workspace_dependencies::NewWorkspaceDependencies;

        #[sqlx::test(
            fixtures("../../tests/fixtures/base.sql",),
            migrations = "../migrations"
        )]
        async fn test_create(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            assert_eq!(
                NewWorkspaceDependencies {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    name: None,
                    description: None,
                    content: "global:rev1".to_owned(),
                }
                .create(&db)
                .await
                .unwrap(),
                1
            );

            assert_eq!(
                NewWorkspaceDependencies {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    name: Some("rrs1".to_owned()),
                    description: None,
                    content: "rrs1:rev1".to_owned(),
                }
                .create(&db)
                .await
                .unwrap(),
                2
            );

            assert!(NewWorkspaceDependencies {
                workspace_id: "test-workspace".into(),
                language: ScriptLang::DuckDb,
                description: None,
                name: None,
                content: "".to_owned(),
            }
            .create(&db)
            .await
            .is_err());

            // Will act as redeployment
            assert_eq!(
                NewWorkspaceDependencies {
                    workspace_id: "test-workspace".into(),
                    language: ScriptLang::Python3,
                    description: None,
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

        #[sqlx::test(
            fixtures("../../tests/fixtures/base.sql",),
            migrations = "../migrations"
        )]
        async fn violate_constraints(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            let db = &db;
            let create = |name| {
                sqlx::query_scalar!(
                    "
                    INSERT INTO workspace_dependencies(name, workspace_id, content, language)
                    VALUES ($1, 'test-workspace', 'test', 'python3') 
                    RETURNING id
                    ",
                    name
                )
                .fetch_one(db)
            };

            assert_eq!(create(Some("test".to_owned())).await.unwrap(), 1);
            assert_eq!(create(None).await.unwrap(), 2);

            assert!(create(Some("test".to_owned())).await.is_err());
            assert!(create(None).await.is_err());
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM workspace_dependencies",)
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                2
            );
            Ok(())
        }
    }

    mod workspace_dependencies {
        // #[sqlx::test(fixtures("../../migrations/20251106152104_workspace_dependencies.up.sql"))]
        // async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
        //     todo!()
        // }
    }
}

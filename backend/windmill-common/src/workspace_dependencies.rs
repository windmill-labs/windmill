use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

use crate::{
    cache::workspace_dependencies::{
        fetch_workspace_dependencies, get_cached_is_unnamed_workspace_dependencies_exists,
        set_cached_is_unnamed_workspace_dependencies_exists,
    },
    error,
    min_version::MIN_VERSION_SUPPORTS_V0_WORKSPACE_DEPENDENCIES,
    scripts::ScriptLang,
    utils::calculate_hash,
    worker::Connection,
};
use phf::phf_set;

pub static BLACKLIST: phf::Set<&'static str> = phf_set! {
    "u/admin/hub_sync",
};

lazy_static::lazy_static! {
    static ref WMDEBUG_FORCE_V0_WORKSPACE_DEPENDENCIES: bool = std::env::var("WMDEBUG_FORCE_V0_WORKSPACE_DEPENDENCIES").is_ok();
}

/// Minimum Windmill version required for workspace dependencies feature
pub const MIN_VERSION_WORKSPACE_DEPENDENCIES: &str = "1.587.0";

pub async fn min_version_supports_v0_workspace_dependencies() -> error::Result<()> {
    // Check if workers support workspace dependencies feature
    if !*WMDEBUG_FORCE_V0_WORKSPACE_DEPENDENCIES
        && !MIN_VERSION_SUPPORTS_V0_WORKSPACE_DEPENDENCIES.met().await
    {
        tracing::warn!(
            "Workspace dependencies feature will be disabled because not all workers support it (minimum version {} required)",
            MIN_VERSION_WORKSPACE_DEPENDENCIES
        );
        return Err(error::Error::WorkersAreBehind {
            feature: "Workspace dependencies".to_string(),
            min_version: MIN_VERSION_WORKSPACE_DEPENDENCIES.to_string(),
        });
    } else {
        Ok(())
    }
}

pub type RawWorkspaceDependencies = std::collections::HashMap<String, String>;

/// Removes workspace dependencies annotation comments from lock files.
/// This is used when passing locks to resolver expect no comments (looking at you, json).
/// IMPORTANT: lock is expected to start with annotations
pub fn clean_lock_from_annotations(lock: &str, language: ScriptLang) -> String {
    let mat = format!("{} workspace-dependencies", language.as_comment_lit());
    lock.lines().filter(|l| !l.starts_with(&mat)).collect()
}

pub fn get_raw_workspace_dependencies(
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
    name: Option<String>,
    language: ScriptLang,
    workspace_id: String,
) -> Option<WorkspaceDependencies> {
    raw_workspace_dependencies_o
        .as_ref()
        .zip(WorkspaceDependencies::to_path(&name, language).ok())
        .and_then(|(hm, path)| hm.get(&path))
        .map(|raw_content| WorkspaceDependencies {
            name,
            workspace_id,
            created_at: chrono::Utc::now(),
            language,
            content: raw_content.to_owned(),
            ..Default::default()
        })
}

fn map_err(e: String) -> error::Error {
    error::Error::FeatureUnavailable(e)
}
#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceDependencies {
    /// Global id (across all workspaces)
    id: i64,
    archived: bool,
    /// If not set becomes default for given language
    pub name: Option<String>,
    pub description: Option<String>,
    pub workspace_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub language: ScriptLang,
    pub content: String,
}

impl WorkspaceDependencies {
    pub fn hash(&self) -> String {
        calculate_hash(&self.content)
    }
    /// Marks workspace dependencies as archived.
    pub async fn archive<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        if language.as_dependencies_filename().is_none() {
            return Ok(());
        }

        sqlx::query!(
            "
            UPDATE workspace_dependencies
            SET archived = true
            WHERE name IS NOT DISTINCT FROM $1 AND workspace_id = $2 AND archived = false AND language = $3
            ",
            name,
            workspace_id,
            language as ScriptLang
        )
        .execute(e)
        .await?;
        Ok(())
    }

    /// Permanently deletes workspace dependencies from the database.
    pub async fn delete<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
        if language.as_dependencies_filename().is_none() {
            return Ok(());
        }

        sqlx::query!(
            "
            DELETE
            FROM workspace_dependencies
            WHERE name IS NOT DISTINCT FROM $1 AND workspace_id = $2 AND language = $3
            ",
            name,
            workspace_id,
            language as ScriptLang
        )
        .execute(e)
        .await?;
        Ok(())
    }

    /// Lists all active workspace dependencies for a workspace.
    pub async fn list<'c>(workspace_id: &str, e: impl PgExecutor<'c>) -> error::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r##"
            SELECT id, created_at, archived, name, description, workspace_id, content, language AS "language: ScriptLang"
                FROM workspace_dependencies
                WHERE archived = false AND workspace_id = $1
            "##,
            workspace_id,
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }

    async fn get_latest_id<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<i64>> {
        tracing::debug!(
            workspace_id = %workspace_id,
            ?language,
            ?name,
            "fetching latest workspace dependencies id"
        );

        // Bunnative and Nativets workspace dependencies go under Bun language
        let language = match language {
            ScriptLang::Nativets | ScriptLang::Bunnative => ScriptLang::Bun,
            l => l,
        };

        let result = sqlx::query_scalar!(
            r#"
            SELECT id FROM workspace_dependencies
            WHERE name IS NOT DISTINCT FROM $1 AND workspace_id = $2 AND archived = false AND language = $3
            LIMIT 1
            "#,
            name,
            workspace_id,
            language as ScriptLang
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)?;

        tracing::debug!(
            workspace_id = %workspace_id,
            ?language,
            ?name,
            ?result,
            "fetched latest workspace dependencies id"
        );
        Ok(result)
    }

    /// Gets the latest version of workspace dependencies by name and language.
    pub async fn get_latest(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        conn: Connection,
    ) -> error::Result<Option<Self>> {
        let Some(dependencies_filename) = language.as_dependencies_filename() else {
            return Ok(None);
        };

        if name.is_none()
            && get_cached_is_unnamed_workspace_dependencies_exists(
                dependencies_filename.clone(),
                workspace_id.to_owned(),
            )
            .map(|exists| exists == false)
            .unwrap_or_default()
        {
            tracing::debug!(
                workspace_id = %workspace_id,
                ?language,
                "skipping unnamed workspace dependencies fetch - cached as non-existent"
            );
            return Ok(None);
        }

        // Fetch from database or HTTP
        let wd = match &conn {
            Connection::Sql(db) => {
                let Some(id) =
                    Self::get_latest_id(name.clone(), language, workspace_id, db).await?
                else {
                    if name.is_none() {
                        set_cached_is_unnamed_workspace_dependencies_exists(
                            dependencies_filename.clone(),
                            workspace_id.to_owned(),
                            false,
                        );
                    }

                    tracing::debug!(
                        workspace_id = %workspace_id,
                        ?language,
                        ?name,
                        "no latest workspace dependencies found"
                    );
                    return Ok(None);
                };
                tracing::debug!(
                    workspace_id = %workspace_id,
                    ?language,
                    ?name,
                    id,
                    "fetching workspace dependencies by id from cache or db"
                );
                Some(fetch_workspace_dependencies(id, workspace_id.to_owned(), db).await?)
            }

            Connection::Http(http_client) => http_client
                .get::<Option<WorkspaceDependencies>>(&format!(
                    "/api/w/{workspace_id}/agent_workers/workspace_dependencies/get_latest/{}{}",
                    language.as_str(),
                    if let Some(ref name_val) = name {
                        format!("?name={name_val}")
                    } else {
                        "".to_owned()
                    }
                ))
                .await
                .map_err(error::Error::from)?,
        };

        if name.is_none() {
            set_cached_is_unnamed_workspace_dependencies_exists(
                dependencies_filename,
                workspace_id.to_owned(),
                wd.is_some(),
            );
        }
        Ok(wd)
    }

    /// Gets workspace dependencies by their unique ID.
    pub async fn get<'c>(
        id: i64,
        workspace_id: String,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT id, content, language AS "language: ScriptLang", name, archived, description, workspace_id, created_at
            FROM workspace_dependencies
            WHERE id = $1 AND workspace_id = $2
            LIMIT 1
            "#,
            id,
            &workspace_id
        )
        .fetch_one(e)
        .await
        .map_err(error::Error::from)
    }

    /// Gets the version history for workspace dependencies.
    pub async fn get_history<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Vec<i64>> {
        if language.as_dependencies_filename().is_none() {
            return Ok(vec![]);
        }
        sqlx::query_scalar!(
            r#"
            SELECT id FROM workspace_dependencies
            WHERE name IS NOT DISTINCT FROM $1 AND workspace_id = $2 AND language = $3
            "#,
            name,
            workspace_id,
            language as ScriptLang
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }

    pub fn to_path(name: &Option<String>, language: ScriptLang) -> error::Result<String> {
        let requirements_filename =
            language
                .as_dependencies_filename()
                .ok_or(error::Error::BadConfig(format!(
                    "workspace dependencies are not supported for: {}",
                    language.as_str()
                )))?;

        Ok(if let Some(name) = name {
            format!("dependencies/{name}.{requirements_filename}")
        } else {
            format!("dependencies/{requirements_filename}")
        })
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceDependenciesPrefetched {
    language: ScriptLang,
    runnable_path: String,
    #[allow(dead_code)]
    workspace_id: String,
    internal: WorkspaceDependenciesPrefetchedInternal,
}

#[derive(Debug, Clone)]
enum WorkspaceDependenciesPrefetchedInternal {
    Explicit(WorkspaceDependenciesAnnotatedRefs<WorkspaceDependencies>),
    Implicit { workspace_dependencies: WorkspaceDependencies, mode: Mode },
    None,
}

impl WorkspaceDependenciesPrefetched {
    pub async fn extract<'c>(
        code: &str,
        language: ScriptLang,
        workspace_id: &str,
        raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
        runnable_path: &str,
        conn: Connection,
    ) -> error::Result<WorkspaceDependenciesPrefetched> {
        use WorkspaceDependenciesPrefetchedInternal::*;

        tracing::debug!(workspace_id, ?language, "extracting workspace dependencies");

        Box::pin(async {
            let r = if let Some(wdar) =
                crate::scripts::extract_workspace_dependencies_annotated_refs(
                    &language,
                    code,
                    runnable_path,
                ) {
                tracing::debug!(workspace_id, ?language, "found explicit annotations");

                let expanded = wdar
                    .expand(language, workspace_id, raw_workspace_dependencies_o, conn)
                    .await?;

                Explicit(expanded)
            // First try in raw dependencies
            } else if let Some(workspace_dependencies) = get_raw_workspace_dependencies(
                raw_workspace_dependencies_o,
                Option::None,
                language,
                workspace_id.to_owned(),
            ) {
                tracing::debug!(
                    workspace_id,
                    ?language,
                    dep_id = workspace_dependencies.id,
                    "using implicit raw"
                );

                // Hardcode to manual for now.
                Implicit { workspace_dependencies, mode: Mode::manual }
            } else if let Some(workspace_dependencies) =
                // If not found, fetch from db
                WorkspaceDependencies::get_latest(
                    Option::None,
                    language,
                    workspace_id,
                    conn,
                )
                .await?
            {
                tracing::debug!(
                    workspace_id,
                    ?language,
                    dep_id = workspace_dependencies.id,
                    "using implicit default"
                );

                // Hardcode to manual for now.
                Implicit { workspace_dependencies, mode: Mode::manual }
            } else {
                tracing::debug!(workspace_id, ?language, "no dependencies found");

                None
            };

            // Crucial part. It will drop all blacklisted runnables
            WorkspaceDependenciesPrefetched {
                internal: r,
                language,
                runnable_path: runnable_path.to_owned(),
                workspace_id: workspace_id.to_owned(),
            }
            .preprocess()
            .await
        })
        .await
    }

    pub fn get_python(&self) -> error::Result<Option<String>> {
        use WorkspaceDependenciesPrefetchedInternal::*;
        Ok(match &self.internal {
            Explicit(wdar @ WorkspaceDependenciesAnnotatedRefs { inline, external, .. }) => {
                tracing::debug!(
                    "Processing explicit workspace dependencies with inline: {}, external count: {}",
                    inline.is_some(),
                    external.len()
                );

                wdar.assert_inline_or_external_or_none().map_err(map_err)?;
                wdar.assert_external_less_than(2).map_err(map_err)?;
                wdar.assert_no_extra_mode_for_external().map_err(map_err)?;
                external
                    .get(0)
                    .map(|wd| wd.content.clone())
                    .or(inline.to_owned())
            }
            Implicit { workspace_dependencies, .. } => {
                self.internal.assert_no_extra_mode().map_err(map_err)?;
                Some(workspace_dependencies.content.to_owned())
            }
            None => Option::None,
        })
    }

    pub fn get_bun(&self) -> error::Result<Option<String>> {
        use WorkspaceDependenciesPrefetchedInternal::*;
        self.internal.assert_no_extra_mode().map_err(map_err)?;
        Ok(match &self.internal {
            Explicit(wdar @ WorkspaceDependenciesAnnotatedRefs { external, .. }) => {
                wdar.assert_no_inline().map_err(map_err)?;
                wdar.assert_external_less_than(2).map_err(map_err)?;
                external
                    .get(0)
                    .map(|wd| wd.content.clone())
                    .or(Some("".to_owned()))
            }
            Implicit { workspace_dependencies, .. } => Some(workspace_dependencies.content.clone()),
            None => Option::None,
        })
    }

    pub fn get_go(&self) -> error::Result<Option<String>> {
        // NOTE: go is disabled for now:
        // https://discord.com/channels/930051556043276338/1031563866641018910/1443541229349634189

        self.internal
            .assert_no_workspace_dependencies()
            .map_err(map_err)?;
        Ok(None)
        // use WorkspaceDependenciesPrefetchedInternal::*;
        // self.internal.assert_no_manual_mode().map_err(map_err)?;
        //         Ok(match &self.internal {
        //             Explicit(wdar @ WorkspaceDependenciesAnnotatedRefs { external, .. }) => {
        //                 wdar.assert_no_inline().map_err(map_err)?;
        //                 wdar.assert_external_less_than(2).map_err(map_err)?;
        //                 external.get(0).map(|wd| dbg!(wd.content.clone())).or(Some(
        //                     "
        // module mymod
        // go 1.25
        // require ()
        //                         "
        //                     .to_owned(),
        //                 ))
        //             }
        //             Implicit { workspace_dependencies, .. } => Some(workspace_dependencies.content.clone()),
        //             None => Option::None,
        //         }
        //         .map(|go_mod_content| {
        //             if let Some(module) = go_mod_content
        //                 .lines()
        //                 .find(|l| l.trim_start().starts_with("module "))
        //             {
        //                 go_mod_content.replace(module, "module mymod")
        //             } else {
        //                 format!("module mymod\n{go_mod_content}")
        //             }
        //         }))
    }

    pub fn get_php(&self) -> error::Result<Option<String>> {
        use WorkspaceDependenciesPrefetchedInternal::*;
        self.internal.assert_no_extra_mode().map_err(map_err)?;
        Ok(match &self.internal {
            Explicit(wdar @ WorkspaceDependenciesAnnotatedRefs { external, .. }) => {
                wdar.assert_no_inline().map_err(map_err)?;
                wdar.assert_external_less_than(2).map_err(map_err)?;
                external
                    .get(0)
                    .map(|wd| wd.content.clone())
                    .or(Some(r#"{"require": {}}"#.to_owned()))
            }
            Implicit { workspace_dependencies, .. } => Some(workspace_dependencies.content.clone()),
            None => Option::None,
        })
    }

    /// Is the runnable permitted to have external references
    pub fn is_external_references_permitted(runnable_path: &str) -> bool {
        !BLACKLIST.contains(runnable_path) && !runnable_path.starts_with("hub/")
    }

    async fn preprocess(mut self) -> error::Result<WorkspaceDependenciesPrefetched> {
        // NOTE: we should error if it is not compatible. User should either update workers or do not use incompatible feature.
        // Check if compatible with legacy and if not check that all workers run compatible versions.
        if let Err(feature) = self.check_legacy_compat() {
            min_version_supports_v0_workspace_dependencies()
                .await
                .map_err(|_| {
                    // NOTE: this error is flakey, sometimes it will error, somethimes it will just ignore.
                    error::Error::WorkersAreBehind {
                        feature,
                        min_version: MIN_VERSION_WORKSPACE_DEPENDENCIES.to_owned(),
                    }
                })?;
        }

        // NOTE: if you update or add new language, add compatibility checks here.
        // for example if you were to update lang do:
        // if let Err(incompatible_e) = self.check_v0_python_compat() {
        //     min_version_supports_v1_workspace_dependencies_python()
        // ...
        // }
        //
        // Where `check_v0_python_compat` describes all features of previous python

        if !Self::is_external_references_permitted(&self.runnable_path) {
            self.remove_external_references();
        }
        Ok(self)
    }

    fn check_legacy_compat(&self) -> Result<(), String> {
        use ScriptLang::*;
        use WorkspaceDependenciesPrefetchedInternal::*;
        match (self.language, &self.internal) {
            // These languages except for python had none of this functionality
            (Php | Bun | Bunnative | Go, wdp) => wdp.assert_no_workspace_dependencies()?,

            // Python, had #(extra_)requirements:
            // but it had no external requirements.
            // that's why we check if it is using only inline syntax
            (Python3, Explicit(wdar)) => wdar.assert_no_external()?,
            (Python3, wdp) => wdp.assert_no_implicit()?,

            (lang @ _, _) => {
                tracing::warn!(
                    self.runnable_path,
                    "skipping workspace dependencies for unsupported language {}",
                    lang.as_str()
                );
                return Ok(());
            }
        }
        Ok(())
    }

    // TODO:
    // pub fn check_v0_compat(&self) -> bool {}
    // pub fn check_v1_compat(&self) -> bool {}
    // pub fn check_v1_python_compat

    fn remove_external_references(&mut self) {
        use WorkspaceDependenciesPrefetchedInternal::*;
        match self.internal {
            Explicit(WorkspaceDependenciesAnnotatedRefs { ref mut external, .. })
                if !external.is_empty() =>
            {
                external.clear();
            }
            // Implicit is an external reference to the default. So we just replace it to none
            ref mut wdp @ Implicit { .. } => drop(std::mem::replace(wdp, None)),
            // Return early not to show warning
            _ => return,
        }
        tracing::warn!(
            self.runnable_path,
            "skipping external workspace dependencies for runnable"
        );
    }

    pub async fn to_lock_header(&self) -> Option<String> {
        use WorkspaceDependenciesPrefetchedInternal::*;

        if min_version_supports_v0_workspace_dependencies()
            .await
            .is_err()
        {
            return Option::None;
        }

        let mut header = vec![];
        let prepend_mode = |mode| {
            format!(
                "{} workspace-dependencies-mode: {}",
                self.language.as_comment_lit(),
                mode
            )
        };

        let insert_line = |hash, name: Option<String>| {
            format!(
                "{} workspace-dependencies: {}:{}",
                self.language.as_comment_lit(),
                name.unwrap_or("default".to_owned()),
                hash
            )
        };
        match &self.internal {
            Explicit(workspace_dependencies_annotated_refs) => {
                header.push(prepend_mode(workspace_dependencies_annotated_refs.mode));
                for wd in &workspace_dependencies_annotated_refs.external {
                    header.push(insert_line(wd.hash(), wd.name.clone()));
                }
            }
            Implicit { workspace_dependencies: wd, mode } => {
                header.push(prepend_mode(*mode));
                header.push(insert_line(wd.hash(), Option::None));
            }
            None => return Option::None,
        }
        Some(header.join("\n"))
    }

    pub fn is_manual(&self) -> bool {
        self.internal.get_mode() == Some(Mode::manual)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceDependenciesAnnotatedRefs<T> {
    /// ```python
    /// # requirements:
    /// # rich==x.y.z    <<
    /// # pandas==x.y.z  <<
    /// ```
    pub inline: Option<String>,
    /// ```python
    /// # requirements: default, base, prod
    ///                 ^^^^^^^  ^^^^  ^^^^
    /// ```
    ///
    /// Can either be a [[String]] or [[WorkspaceDependencies]]
    ///
    /// The workflow is following:
    /// 1. You create Self with <[[String]]> - this will fetch a minimal amount of info (just the name).
    /// 2. You [[Self::expand]] to replace all external names with <[[WorkspaceDependencies]]>
    pub external: Vec<T>,
    pub mode: Mode,
}

/// `# extra_requirements:` - Extra
/// `# requirements:` - Manual
#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, strum_macros::Display, Clone, Copy, Debug)]
pub enum Mode {
    manual,
    extra,
}

impl WorkspaceDependenciesPrefetchedInternal {
    fn get_mode(&self) -> Option<Mode> {
        use WorkspaceDependenciesPrefetchedInternal::*;
        match &self {
            Explicit(WorkspaceDependenciesAnnotatedRefs { mode, .. }) | Implicit { mode, .. } => {
                Some(*mode)
            }
            None => Option::None,
        }
    }

    fn assert_no_implicit(&self) -> Result<(), String> {
        if matches!(self, Self::Implicit { .. }) {
            Err(format!("'default workspace dependencies'"))
        } else {
            Ok(())
        }
    }

    fn assert_no_workspace_dependencies(&self) -> Result<(), String> {
        if !matches!(self, Self::None { .. }) {
            Err(format!("'workspace dependencies'"))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn assert_no_explicit(&self) -> Result<(), String> {
        if matches!(self, Self::Explicit { .. }) {
            Err(format!("'external workspace dependencies'"))
        } else {
            Ok(())
        }
    }

    fn assert_no_extra_mode(&self) -> Result<(), String> {
        if self.get_mode() == Some(Mode::extra) {
            Err(format!("'workspace dependencies in extra mode'"))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn assert_no_manual_mode(&self) -> Result<(), String> {
        if self.get_mode() == Some(Mode::manual) {
            Err(format!("'workspace dependencies in manual mode'"))
        } else {
            Ok(())
        }
    }
}

impl WorkspaceDependenciesAnnotatedRefs<WorkspaceDependencies> {
    fn assert_no_inline(&self) -> Result<(), String> {
        if self.inline.is_none() {
            Ok(())
        } else {
            Err(format!("'inline workspace dependencies'"))
        }
    }

    fn assert_no_extra_mode_for_external(&self) -> Result<(), String> {
        if self.mode == Mode::extra && !self.external.is_empty() {
            Err(format!("'external workspace dependencies in extra mode'"))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn assert_no_extra_mode_for_inline(&self) -> Result<(), String> {
        if self.mode == Mode::extra && self.inline.is_some() {
            Err(format!("'inline workspace dependencies in extra mode'"))
        } else {
            Ok(())
        }
    }

    fn assert_no_external(&self) -> Result<(), String> {
        self.assert_external_less_than(1)
            .map_err(|_e| format!("'external workspace dependencies'"))
    }

    fn assert_inline_or_external_or_none(&self) -> Result<(), String> {
        if self.inline.is_some() && !self.external.is_empty() {
            Err(format!(
                "'inline and externally referenced workspace dependencies at the same time'"
            ))
        } else {
            Ok(())
        }
    }
    fn assert_external_less_than(&self, amount: usize) -> Result<(), String> {
        if self.external.len() < amount {
            Ok(())
        } else {
            Err(format!(
                "'multiple external workspace dependencies referenced'",
            ))
        }
    }
}

impl WorkspaceDependenciesAnnotatedRefs<String> {
    pub(super) async fn expand(
        self,
        language: ScriptLang,
        workspace_id: &str,
        raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
        conn: Connection,
    ) -> error::Result<WorkspaceDependenciesAnnotatedRefs<WorkspaceDependencies>> {
        let mut res = WorkspaceDependenciesAnnotatedRefs {
            inline: self.inline,
            external: vec![],
            mode: self.mode,
        };

        for name in self.external {
            // "default" maps to unnamed workspace dependencies.
            let name = if name == "default" { None } else { Some(name) };
            // First try in raw dependencies
            if let Some(wd) = get_raw_workspace_dependencies(
                raw_workspace_dependencies_o,
                name.clone(),
                language,
                workspace_id.to_owned(),
            ) {
                res.external.push(wd);

            // If not found, fetch from db
            } else if let Some(wd) = WorkspaceDependencies::get_latest(
                name.clone(),
                language,
                workspace_id,
                conn.clone(),
            )
            .await?
            {
                res.external.push(wd.clone());
            } else {
                tracing::warn!(
                    workspace_id,
                    ?language,
                    dependency_name = name,
                    "workspace dependencies not found"
                );
            }
        }
        Ok(res)
    }
    // TODO: Maybe implemented by our Annotations macro
    // TODO: Add sep config ':' or '='?
    pub fn parse(
        comment: &str,
        keyword: &str,
        code: &str,
        validity_re_o: Option<&Regex>,
        runnable_path: &str,
    ) -> Option<Self> {
        let (extra_deps, manual_deps) = (format!("extra_{keyword}:"), format!("{keyword}:"));

        let Some((pos, mat)) = code.lines().find_position(|l| {
            l.starts_with(&comment) && (l.contains(&extra_deps) || l.contains(&manual_deps))
        }) else {
            return None;
        };
        let mut lines_it = code.lines().skip(pos);

        let mode = if mat.contains(&extra_deps) {
            Mode::extra
        } else {
            Mode::manual
        };

        let external = {
            let next_line = lines_it.next();
            if !WorkspaceDependenciesPrefetched::is_external_references_permitted(runnable_path) {
                tracing::warn!(
                    runnable_path,
                    "skipping external workspace dependencies for runnable"
                );

                Default::default()
                // return Err(error::Error::BadConfig(format!(
                //     "{runnable_path} should not include external Workspace Dependencies"
                // )));
            } else {
                next_line
                    .map(|s| {
                        match mode {
                            Mode::manual => s.replace(&manual_deps, ""),
                            Mode::extra => s.replace(&extra_deps, ""),
                        }
                        .replace(comment, "")
                    })
                    .map(|unparsed| {
                        unparsed
                            .split(',')
                            // TODO: do we want to sort it?
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            // .map(FromName::from_name)
                            .map(str::to_owned)
                            .collect_vec()
                    })
                    .unwrap_or_default()
            }
        };

        let inline_deps = lines_it
            .map_while(|l| {
                match validity_re_o {
                    Some(re) => re.captures(l).and_then(|c| c.get(1).map(|m| m.as_str())),
                    None => {
                        if !l.starts_with(comment) {
                            None
                        } else {
                            // Skip comment
                            // If it fails (None) iteration is just finished.
                            l.get(comment.len()..)
                        }
                    }
                }
            })
            .join("\n");

        let inline = if inline_deps.trim().is_empty() {
            None
        } else {
            Some(inline_deps)
        };

        Some(WorkspaceDependenciesAnnotatedRefs {
            inline, // TODO: Parse
            external,
            mode,
        })
    }
}

#[cfg(test)]
mod workspace_dependencies_tests {
    use super::*;

    #[test]
    fn test_parse_annotation_python_requirements_manual_mode() {
        let code = r#"
# requirements:  default,   base
#requests==2.31.0
#pandas>=1.5.0

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(
            result.external,
            vec!["default".to_owned(), "base".to_owned()]
        );
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "requests==2.31.0\npandas>=1.5.0"
        );
    }

    #[test]
    fn test_parse_annotation_python_extra_requirements_mode() {
        let code = r#"
# extra_requirements: utils
#numpy>=1.24.0

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::extra));
        assert_eq!(result.external, vec!["utils".to_owned()]);
        assert_eq!(result.inline.as_ref().unwrap(), "numpy>=1.24.0");
    }

    #[test]
    fn test_parse_annotation_typescript_requirements() {
        let code = r#"
// requirements: utils, base
//{
//  "dependencies": {
//    "axios": "^1.6.0"
//  }
//}

export function main() {}
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "//",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["utils".to_owned(), "base".to_owned()]);
        let expected_inline = r#"{
  "dependencies": {
    "axios": "^1.6.0"
  }
}"#;
        assert_eq!(result.inline.as_ref().unwrap(), expected_inline);
    }

    #[test]
    fn test_parse_annotation_with_spacing_variations() {
        let code = r#"
#requirements: no_space
def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["no_space".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_with_spacing_variations_spaced() {
        let code = r#"
# requirements: with_space
def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["with_space".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_go_style() {
        let code = r#"
// go_mod:   base,
//github.com/gin-gonic/gin v1.9.1

package main
func main() {}
"#;

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("//", "go_mod", code, None, "")
                .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["base".to_owned()]);
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "github.com/gin-gonic/gin v1.9.1"
        );
    }

    #[test]
    fn test_parse_annotation_no_inline_deps() {
        let code = r#"
# requirements: default

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["default".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_inline_only() {
        let code = r#"
# requirements:
#requests==2.31.0
# pandas>=1.5.0

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert!(result.external.is_empty());
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "requests==2.31.0\n pandas>=1.5.0"
        );
    }

    #[test]
    fn test_parse_annotation_no_match() {
        let code = r#"
def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_annotation_php_style() {
        let code = r#"
<?php
// requirements: composer
//{
//  "require": {
//    "guzzlehttp/guzzle": "^7.0"
//  }
//}

function main() {}
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "//",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert_eq!(result.external, vec!["composer".to_owned()]);
        let expected_inline = r#"{
  "require": {
    "guzzlehttp/guzzle": "^7.0"
  }
}"#;
        assert_eq!(result.inline.as_ref().unwrap(), expected_inline);
    }

    #[test]
    fn test_parse_annotation_just_requirements_colon() {
        let code = r#"
#requirements:

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert!(result.external.is_empty());
        assert!(result.inline.is_none());
    }
    #[test]
    fn test_parse_annotation_blacklisted() {
        let code = r#"
#requirements: hello, world

def main():
    pass
"#;

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse(
            "#",
            "requirements",
            code,
            None,
            "u/admin/hub_sync",
        )
        .unwrap();
        assert!(matches!(result.mode, Mode::manual));
        assert!(result.external.is_empty());
        assert!(result.inline.is_none());
    }
}

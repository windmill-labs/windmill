pub mod scoped_dependency_map;
pub mod trigger_dependents;
pub mod workspace_dependencies;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use serde_json::value::RawValue;
use sqlx::types::Json;
use uuid::Uuid;
use windmill_common::error;
use windmill_common::scripts::ScriptLang;
use windmill_common::utils::WarnAfterExt;
use windmill_common::workspace_dependencies::{
    WorkspaceDependencies, WorkspaceDependenciesPrefetched,
};
use windmill_parser_ts::parse_expr_for_imports;

fn try_normalize(path: &Path) -> Option<PathBuf> {
    let mut ret = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(..) | Component::RootDir => return None,
            Component::CurDir => {}
            Component::ParentDir => {
                if !ret.pop() {
                    return None;
                }
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }

    Some(ret)
}

fn parse_ts_relative_imports(
    raw_code: &str,
    script_path: &str,
) -> windmill_common::error::Result<Vec<String>> {
    let mut relative_imports = vec![];
    let r = parse_expr_for_imports(raw_code, true)?;
    for import in r {
        let import = import.trim_end_matches(".ts");
        if import.starts_with("/") {
            relative_imports.push(import.trim_start_matches("/").to_string());
        } else if import.starts_with(".") {
            let normalized = try_normalize(std::path::Path::new(&format!(
                "{}/../{}",
                script_path, import
            )));
            if let Some(normalized) = normalized {
                let normalized = normalized.to_str().unwrap().to_string();
                relative_imports.push(normalized);
            } else {
                tracing::error!("error canonicalizing path: {script_path} with import {import}");
            }
        }
    }

    Ok(relative_imports)
}

pub fn extract_relative_imports(
    raw_code: &str,
    script_path: &str,
    language: &Option<ScriptLang>,
) -> Option<Vec<String>> {
    match language {
        #[cfg(feature = "python")]
        Some(ScriptLang::Python3) => {
            windmill_parser_py_imports::parse_relative_imports(&raw_code, script_path).ok()
        }
        Some(ScriptLang::Bun) | Some(ScriptLang::Bunnative) | Some(ScriptLang::Deno) => {
            parse_ts_relative_imports(&raw_code, script_path).ok()
        }
        _ => None,
    }
}

pub fn extract_referenced_paths(
    raw_code: &str,
    script_path: &str,
    language: Option<ScriptLang>,
) -> Option<Vec<String>> {
    let mut referenced_paths = vec![];
    if let Some(wk_deps_refs) = language
        .and_then(|l| windmill_common::scripts::extract_workspace_dependencies_annotated_refs(&l, raw_code, script_path))
        .map(|r| r.external)
    {
        let l = language.expect("should be some");
        for wk_deps_ref in wk_deps_refs {
            if let Some(path) = WorkspaceDependencies::to_path(&Some(wk_deps_ref), l).ok() {
                referenced_paths.push(path);
            };
        }
    } else if let (Some(l), true /* Only if it is not blacklisted */) = (
        language,
        WorkspaceDependenciesPrefetched::is_external_references_permitted(script_path),
    ) {
        // we assume all runnables without annotated dependencies reference default dependencies file.
        WorkspaceDependencies::to_path(&None, l)
            .ok()
            .inspect(|p| referenced_paths.push(p.to_owned()));
    }

    if let Some(relative_imports) = extract_relative_imports(raw_code, script_path, &language) {
        referenced_paths.extend(relative_imports);
    }

    if referenced_paths.is_empty() {
        None
    } else {
        Some(referenced_paths)
    }
}

pub async fn process_relative_imports(
    db: &sqlx::Pool<sqlx::Postgres>,
    _job_id: Option<Uuid>,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    w_id: &str,
    script_path: &str,
    parent_path: Option<String>,
    deployment_message: Option<String>,
    code: &str,
    script_lang: &Option<ScriptLang>,
    permissioned_as_email: &str,
    created_by: &str,
    permissioned_as: &str,
) -> error::Result<()> {
    use scoped_dependency_map::ScopedDependencyMap;
    use trigger_dependents::trigger_dependents_to_recompute_dependencies;

    // TODO: Should be moved into handle_dependency_job body to be more consistent with how flows and apps are handled
    {
        let mut tx = db.begin().await?;
        let mut dependency_map = ScopedDependencyMap::fetch_maybe_rearranged(
            &w_id,
            script_path,
            "script",
            &parent_path,
            db,
        )
        .await?;

        tx = dependency_map
            .patch(
                extract_referenced_paths(&code, script_path, *script_lang),
                // Ideally should be None, but due to current implementation will use empty string to represent None.
                "".into(),
                tx,
            )
            .await?;

        dependency_map.dissolve(tx).await.commit().await?;
    }

    {
        let mut already_visited = args
            .map(|x| {
                x.get("already_visited")
                    .map(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
                    .flatten()
            })
            .flatten()
            .unwrap_or_default();

        let importers = ScopedDependencyMap::get_dependents(script_path, w_id, db).await?;

        already_visited.push(script_path.to_string());
        match tokio::time::timeout(
            core::time::Duration::from_secs(60),
            Box::pin(trigger_dependents_to_recompute_dependencies(
                w_id,
                importers,
                deployment_message,
                parent_path,
                permissioned_as_email,
                created_by,
                permissioned_as,
                db,
                already_visited,
            )),
        )
        .warn_after_seconds(10)
        .await
        {
            Ok(Err(e)) => {
                tracing::error!(%e, "error triggering dependents to recompute dependencies")
            }
            Err(e) => {
                tracing::error!(%e, "triggering dependents to recompute dependencies has timed out")
            }
            _ => {}
        }
    }

    Ok(())
}

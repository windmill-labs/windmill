use itertools::Itertools;
use serde::Serialize;
use sqlx::PgExecutor;

use crate::{error, scripts::ScriptLang, utils::calculate_hash, worker::Connection};

// TODO: Make sure there is only one archived
// and only one or none unnamed for given language
#[derive(sqlx::FromRow, Debug, Clone, Serialize, Default)]
pub struct WorkspaceDependencies {
    /// Global id (accross all workspaces)
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
    // TODO: Make sure it all starts with 1
    pub fn hash(&self) -> String {
        if self.id == 0 {
            calculate_hash(&self.content)
        } else {
            self.id.to_string()
        }
    }
    /// Marks workspace dependencies as archived.
    pub async fn archive<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<()> {
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

    /// Gets the latest version of workspace dependencies by name and language.
    pub async fn get_latest(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        conn: Connection,
    ) -> error::Result<Option<Self>> {
        match conn {
            Connection::Sql(db) => sqlx::query_as!(
                Self,
                r#"
                SELECT id, content, language AS "language: ScriptLang", name, description, archived, workspace_id, created_at
                FROM workspace_dependencies
                WHERE name IS NOT DISTINCT FROM $1 AND workspace_id = $2 AND archived = false AND language = $3
                LIMIT 1
                "#,
                name,
                workspace_id,
                language as ScriptLang
            )
            .fetch_optional(&db)
            .await
            .map_err(error::Error::from),

            // TODO: Check it works for non admin when endpoint is admin only.
            // Connection::Http(http_client) => http_client.get(format!(), headers, body),
            Connection::Http(http_client) => todo!(),
        }
    }

    /// Gets workspace dependencies by their unique ID.
    pub async fn get<'c>(
        id: i64,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT id, content, language AS "language: ScriptLang", name, archived, description, workspace_id, created_at
            FROM workspace_dependencies
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

    /// Gets the version history for workspace dependencies.
    pub async fn get_history<'c>(
        name: Option<String>,
        language: ScriptLang,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> error::Result<Vec<i64>> {
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
                    "raw requirements are not supported for: {}",
                    language.as_str()
                )))?;

        Ok(if let Some(name) = name {
            format!("dependencies/{name}.{requirements_filename}")
        } else {
            format!("dependencies/{requirements_filename}")
        })
    }
}

pub enum WorkspaceDependenciesPrefetched {
    Explicit(WorkspaceDependenciesAnnotatedRefs<WorkspaceDependencies>),
    Implicit { workspace_dependencies: WorkspaceDependencies, mode: Mode },
    None,
}

impl WorkspaceDependenciesPrefetched {
    pub fn get_one_external_only_manual(
        &self,
        // just for logging
        workspace_id: &str,
        // just for logging
        script_path: Option<String>,
    ) -> Option<String> {
        use WorkspaceDependenciesPrefetched::*;

        if self.get_mode() == Some(Mode::Extra) {
            tracing::warn!(
                workspace_id,
                script_path,
                "extra mode is not supported yet, this may change in future"
            );
            return Option::None;
        }

        match &self {
            Explicit(WorkspaceDependenciesAnnotatedRefs { inline, external, .. }) => {
                if inline.is_some() {
                    tracing::warn!(workspace_id, script_path, "inline workspace dependencies are ignored, this could be implemented later");
                }
                if external.len() > 1 {
                    tracing::warn!(workspace_id, script_path, "multiple external workspace dependencies found, all except first are ignored, this might be supported eventually");
                }

                external.get(0).map(|wd| wd.content.clone())
            }
            Implicit { workspace_dependencies, .. } => Some(workspace_dependencies.content.clone()),
            None => Option::None,
        }
    }
    pub fn get_mode(&self) -> Option<Mode> {
        match self {
            WorkspaceDependenciesPrefetched::Explicit(WorkspaceDependenciesAnnotatedRefs {
                mode,
                ..
            })
            | WorkspaceDependenciesPrefetched::Implicit { mode, .. } => Some(*mode),
            WorkspaceDependenciesPrefetched::None => Option::None,
        }
    }

    pub async fn extract<'c>(
        code: &str,
        language: ScriptLang,
        workspace_id: &str,
        raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
        conn: Connection,
    ) -> error::Result<Self> {
        tracing::debug!(workspace_id, ?language, "extracting workspace dependencies");
        Ok(
            if let Some(wdar) = language.extract_workspace_dependencies_annotated_refs(code) {
                tracing::debug!(workspace_id, ?language, "found explicit annotations");

                Self::Explicit(
                    wdar.expand(language, workspace_id, raw_workspace_dependencies_o, conn)
                        .await?,
                )
            // First try in raw dependencies
            } else if let Some(workspace_dependencies) = get_raw_workspace_dependencies(
                raw_workspace_dependencies_o,
                None,
                language,
                workspace_id.to_owned(),
            ) {
                tracing::debug!(
                    workspace_id,
                    ?language,
                    dep_id = workspace_dependencies.id,
                    "using implicit raw"
                );

                Self::Implicit { workspace_dependencies, mode: Mode::Manual } // Hardcode to manual for now.
            } else if let Some(workspace_dependencies) =
                // If not found, fetch from db
                WorkspaceDependencies::get_latest(None, language, workspace_id, conn)
                        .await?
            {
                tracing::debug!(
                    workspace_id,
                    ?language,
                    dep_id = workspace_dependencies.id,
                    "using implicit default"
                );

                Self::Implicit { workspace_dependencies, mode: Mode::Manual } // Hardcode to manual for now.
            } else {
                tracing::debug!(workspace_id, ?language, "no dependencies found");

                Self::None
            },
        )
    }
    pub async fn to_lock_header(&self, language: ScriptLang) -> error::Result<Option<String>> {
        use WorkspaceDependenciesPrefetched::*;

        let mut header = vec![];
        let prepend_mode = |mode| {
            format!(
                "{} workspace-dependencies-mode: {}",
                language.as_comment_lit(),
                mode
            )
        };

        let insert_line = |hash, name: Option<String>| {
            format!(
                "{} workspace-dependencies: {}:{}",
                language.as_comment_lit(),
                name.unwrap_or("default".to_owned()),
                hash
            )
        };
        match self {
            Explicit(workspace_dependencies_annotated_refs) => {
                // TODO: error on extra for now
                header.push(prepend_mode(workspace_dependencies_annotated_refs.mode));
                for wd in &workspace_dependencies_annotated_refs.external {
                    header.push(insert_line(wd.hash(), wd.name.clone()));
                }
            }
            Implicit { workspace_dependencies: wd, mode } => {
                // TODO: error on extra for now
                header.push(prepend_mode(*mode));
                header.push(insert_line(wd.hash(), Option::None));
            }
            None => return Ok(Option::None),
        }
        Ok(Some(header.join("\n")))
    }
}

pub struct WorkspaceDependenciesAnnotatedRefs<T: Container> {
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
    /// 1. You create Self with <[[String]]> - this will fetch a minimal amount of info.
    /// 2. You [[Self::expand]] to replace all external names with <[[WorkspaceDependencies]]>
    pub external: Vec<T::Ty>,
    pub mode: Mode,
}

/// Trait allowing the same struct to hold either String names (fast parsing without DB)
/// or resolved WorkspaceDependencies objects (after DB expansion).
pub trait Container {
    // TODO(#29661): Use default associated type
    type Ty;
}
impl Container for String {
    type Ty = Self;
}
impl Container for WorkspaceDependencies {
    type Ty = Self;
}

/// `# extra_requirements:` - Extra
/// `# requirements:` - Manual
#[derive(PartialEq, Eq, strum_macros::Display, Clone, Copy)]
pub enum Mode {
    Manual,
    Extra,
}

pub type RawWorkspaceDependencies = std::collections::HashMap<String, String>;

pub fn get_raw_workspace_dependencies(
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
    name: Option<String>,
    language: ScriptLang,
    workspace_id: String,
) -> Option<WorkspaceDependencies> {
    dbg!(dbg!(raw_workspace_dependencies_o.as_ref())
        .zip(dbg!(WorkspaceDependencies::to_path(&name, language).ok()))
        .and_then(|(hm, path)| hm.get(&path))
        .map(|raw_content| WorkspaceDependencies {
            // TODO: Name should be different to prevent from collisions and pg index violation
            name,
            workspace_id,
            created_at: chrono::Utc::now(),
            language,
            content: raw_content.to_owned(),
            ..Default::default()
        }))
}

impl<T: Container<Ty = String>> WorkspaceDependenciesAnnotatedRefs<T> {
    pub async fn expand(
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
            // First try in raw dependencies
            if let Some(wd) = get_raw_workspace_dependencies(
                raw_workspace_dependencies_o,
                Some(name.clone()),
                language,
                workspace_id.to_owned(),
            ) {
                // TODO: Test description is correct.
                res.external.push(wd);

            // If not found, fetch from db
            } else if let Some(wd) = WorkspaceDependencies::get_latest(
                Some(name.clone()),
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
                    dependency_name = %name,
                    "workspace dependency not found"
                );
            }
        }
        Ok(res)
    }
    // TODO: manual should take precendence over extra
    // TODO: Maybe implemented by our Annotations macro
    // TODO: BREAKING: Note this will include all seqsequent lines as long as they start with comment.
    // TODO: What sep should be? ':' or '='?
    pub fn parse(comment: &str, keyword: &str, code: &str) -> Option<Self> {
        let (extra_deps, manual_deps) = (format!("extra_{keyword}:"), format!("{keyword}:"));

        let Some((pos, mat)) = code.lines().find_position(|l| {
            l.starts_with(&comment) && (l.contains(&extra_deps) || l.contains(&manual_deps))
        }) else {
            return None;
        };
        let mut lines_it = code.lines().skip(pos);

        let mode = if mat.contains(&extra_deps) {
            Mode::Extra
        } else {
            Mode::Manual
        };

        let external = lines_it
            .next()
            // We are not interested in matched annotation.
            .map(|s| {
                match mode {
                    Mode::Manual => s.replace(&manual_deps, ""),
                    Mode::Extra => s.replace(&extra_deps, ""),
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
            .unwrap_or_default();

        // dbg!(&external);

        let inline_deps = lines_it
            .map_while(|l| {
                if !l.starts_with(comment) {
                    None
                } else {
                    // Skip comment
                    // If it fails (None) iteration is just finished.
                    l.get(comment.len()..)
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Extra));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("//", "requirements", code)
                .unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("//", "go_mod", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result = WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code);
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("//", "requirements", code)
                .unwrap();
        assert!(matches!(result.mode, Mode::Manual));
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

        let result =
            WorkspaceDependenciesAnnotatedRefs::<String>::parse("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert!(result.external.is_empty());
        assert!(result.inline.is_none());
    }
}

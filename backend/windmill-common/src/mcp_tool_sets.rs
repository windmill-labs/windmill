//! Named MCP tool sets: a workspace admin's selection of scripts and flows, served at its own
//! MCP URL. A set narrows and never grants: a member signed in to it gets the runnables of the
//! set they can see and run themselves.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    db::DB,
    error::{Error, Result},
};

/// Marks a token signed in to a tool set: `mcp:set:<name>`.
pub const SET_SCOPE_PREFIX: &str = "mcp:set:";

const MAX_NAME_LEN: usize = 50;
const MAX_INSTRUCTIONS_LEN: usize = 4000;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpToolSet {
    /// Script paths: `*`, an exact path, or a `<prefix>/*` subtree.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Flow paths, as for scripts.
    #[serde(default)]
    pub flows: Vec<String>,
    /// Added to the server's instructions for clients connected to this set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Also serve `list_runs` and `get_run`, confined to the runs of the set's scripts and flows.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub run_history: bool,
}

/// What a caller signed in to a tool set is confined to, read from its expanded scopes.
pub struct ToolSetSelection {
    scripts: Vec<String>,
    flows: Vec<String>,
}

/// `None` for a caller signed in to no tool set.
pub fn tool_set_selection(scopes: &[String]) -> Option<ToolSetSelection> {
    tool_set_of(scopes)?;
    let patterns = |prefix: &str| -> Vec<String> {
        scopes
            .iter()
            .filter_map(|s| s.strip_prefix(prefix))
            .flat_map(|list| list.split(','))
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    };
    Some(ToolSetSelection { scripts: patterns("mcp:scripts:"), flows: patterns("mcp:flows:") })
}

impl ToolSetSelection {
    /// Whether a run of `kind` (`script` or `flow`, as jobs name them) at `path` is in the set.
    pub fn allows(&self, kind: &str, path: &str) -> bool {
        let patterns = match kind {
            "script" => &self.scripts,
            "flow" => &self.flows,
            _ => return false,
        };
        patterns.iter().any(|p| {
            p == "*"
                || p == path
                || p.strip_suffix("/*").is_some_and(|prefix| {
                    path.strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with('/'))
                })
        })
    }

    /// The runs queries covering the set, as `(job kind, path filter)`: a page of one query is
    /// then rarely emptied by `allows` before it reaches the caller. Each kind gets its own, so a
    /// scripts-only set is not crowded out by the folder's flow runs; its exact paths and subtree
    /// prefixes (`f/ops/`) are queried apart, as the jobs API ANDs the two filters and a prefix
    /// for an exact path would also match its longer siblings. `None`: every path of that kind.
    pub fn run_queries(&self) -> Vec<(&'static str, Option<(&'static str, Vec<String>)>)> {
        let mut queries = Vec::new();
        for (kind, patterns) in [("script", &self.scripts), ("flow", &self.flows)] {
            if patterns.iter().any(|p| p == "*") {
                queries.push((kind, None));
                continue;
            }
            let (subtrees, exact): (Vec<&String>, Vec<&String>) =
                patterns.iter().partition(|p| p.ends_with("/*"));
            if !exact.is_empty() {
                let exact = exact.into_iter().cloned().collect();
                queries.push((kind, Some(("script_path_exact", exact))));
            }
            if !subtrees.is_empty() {
                let prefixes = subtrees.iter().map(|p| p.trim_end_matches('*').to_string());
                queries.push((kind, Some(("script_path_start", prefixes.collect()))));
            }
        }
        queries
    }
}

/// By name, as stored in `workspace_settings.mcp_tool_sets`.
pub type McpToolSets = BTreeMap<String, McpToolSet>;

pub fn validate_tool_set_name(name: &str) -> Result<()> {
    let usable = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if usable {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "`{name}` is not a usable tool set name: use up to {MAX_NAME_LEN} lowercase letters, digits, `_` and `-`"
        )))
    }
}

pub fn validate_tool_sets(sets: &McpToolSets) -> Result<()> {
    for (name, set) in sets {
        validate_tool_set_name(name)?;
        if set.scripts.is_empty() && set.flows.is_empty() {
            return Err(Error::BadRequest(format!(
                "tool set `{name}` selects no scripts or flows"
            )));
        }
        for path in set.scripts.iter().chain(&set.flows) {
            // Paths are joined with commas into selection scopes, and only `*`, an exact path
            // and a trailing `/*` are matched.
            let base = path.strip_suffix("/*").unwrap_or(path);
            let usable = path == "*"
                || (!base.is_empty()
                    && !base.contains(['*', ',', ' '])
                    && ["u/", "f/", "g/"].iter().any(|p| base.starts_with(p)));
            if !usable {
                return Err(Error::BadRequest(format!(
                    "`{path}` in tool set `{name}` is not a script or flow path: use `*`, a path, or `<folder path>/*`"
                )));
            }
        }
        if set
            .instructions
            .as_ref()
            .is_some_and(|i| i.len() > MAX_INSTRUCTIONS_LEN)
        {
            return Err(Error::BadRequest(format!(
                "the instructions of tool set `{name}` are longer than {MAX_INSTRUCTIONS_LEN} characters"
            )));
        }
    }
    Ok(())
}

/// Read with the server's pool, whoever asks: callers must have authenticated the request for
/// `w_id` already. A set only narrows the token it is read for, so it grants nothing by itself.
pub async fn get_tool_set(db: &DB, w_id: &str, name: &str) -> Result<Option<McpToolSet>> {
    let value = sqlx::query_scalar!(
        "SELECT mcp_tool_sets -> $2 FROM workspace_settings WHERE workspace_id = $1",
        w_id,
        name
    )
    .fetch_optional(db)
    .await?
    .flatten();
    value
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| Error::internal_err(format!("reading tool set `{name}`: {e}")))
}

/// The set a token is signed in to, if any.
pub fn tool_set_of(scopes: &[String]) -> Option<&str> {
    scopes.iter().find_map(|s| s.strip_prefix(SET_SCOPE_PREFIX))
}

/// The scopes a token signed in to set `name` acts with: the set's current selection in place of
/// whatever selection it carried, and every other scope kept, as each only narrows it
/// (`mcp:read_only`, `mcp:workspaces:`, a tag fence). `None` for a set that no longer exists.
pub fn expand_tool_set_scopes(
    held: &[String],
    name: &str,
    set: Option<&McpToolSet>,
) -> Vec<String> {
    const SELECTING: &[&str] = &[
        "mcp:all",
        "mcp:favorites",
        "mcp:scripts:",
        "mcp:flows:",
        "mcp:endpoints:",
        "mcp:hub:",
        SET_SCOPE_PREFIX,
        "mcp:runnables_as_tools",
    ];
    let kept = held
        .iter()
        .filter(|s| !SELECTING.iter().any(|p| s.starts_with(p)))
        .cloned();
    let selection = match set {
        Some(set) => tool_set_scopes(name, set),
        None => vec![format!("{SET_SCOPE_PREFIX}{name}")],
    };
    kept.chain(selection).collect()
}

/// What a token signed in to `name` acts with: the set as a selection of scripts and flows, with
/// no endpoint tools.
fn tool_set_scopes(name: &str, set: &McpToolSet) -> Vec<String> {
    let mut scopes = vec![
        format!("{SET_SCOPE_PREFIX}{name}"),
        "mcp:runnables_as_tools".to_string(),
    ];
    if !set.scripts.is_empty() {
        scopes.push(format!("mcp:scripts:{}", set.scripts.join(",")));
    }
    if !set.flows.is_empty() {
        scopes.push(format!("mcp:flows:{}", set.flows.join(",")));
    }
    scopes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(paths: &[&str]) -> McpToolSets {
        BTreeMap::from([(
            "support".to_string(),
            McpToolSet {
                scripts: paths.iter().map(|p| p.to_string()).collect(),
                flows: vec![],
                instructions: None,
                run_history: false,
            },
        )])
    }

    #[test]
    fn paths_are_patterns_the_matchers_read() {
        assert!(validate_tool_sets(&set(&["*", "f/ops/*", "u/alice/report"])).is_ok());
        for bad in ["f/ops/*/x", "f/a,f/b", "ops/x", "f/*x", ""] {
            assert!(validate_tool_sets(&set(&[bad])).is_err(), "{bad}");
        }
        assert!(validate_tool_sets(&set(&[])).is_err());
        assert!(validate_tool_set_name("Support").is_err());
    }

    #[test]
    fn expansion_replaces_the_selection_and_keeps_every_narrowing() {
        let held: Vec<String> = [
            "mcp:set:support",
            "mcp:all",
            "mcp:scripts:*",
            "mcp:read_only",
            "mcp:workspaces:a",
            "if_jobs:filter_tags:deno",
        ]
        .map(String::from)
        .to_vec();
        let sets = set(&["f/ops/*"]);
        let expanded = expand_tool_set_scopes(&held, "support", sets.get("support"));
        assert_eq!(
            expanded,
            [
                "mcp:read_only",
                "mcp:workspaces:a",
                "if_jobs:filter_tags:deno",
                "mcp:set:support",
                "mcp:runnables_as_tools",
                "mcp:scripts:f/ops/*"
            ]
        );
        assert_eq!(
            expand_tool_set_scopes(&held, "support", None),
            [
                "mcp:read_only",
                "mcp:workspaces:a",
                "if_jobs:filter_tags:deno",
                "mcp:set:support"
            ]
        );
    }

    #[test]
    fn run_tools_reach_only_the_sets_runs() {
        let scopes: Vec<String> = [
            "mcp:set:support",
            "mcp:scripts:f/ops/*,f/a/x",
            "mcp:flows:f/a/sync",
        ]
        .map(String::from)
        .to_vec();
        let set = tool_set_selection(&scopes).unwrap();
        assert!(set.allows("script", "f/ops/check"));
        assert!(set.allows("flow", "f/a/sync"));
        // The prefixes a runs query is narrowed by match these; `allows` must not.
        assert!(!set.allows("script", "f/a/xy"));
        assert!(!set.allows("script", "f/opsx/check"));
        assert!(!set.allows("flow", "f/a/x"));
        assert!(!set.allows("singlestepflow", "f/ops/check"));
        assert!(tool_set_selection(&["mcp:scripts:f/ops/*".to_string()]).is_none());
        // Each kind queries its own paths, and an exact path is never queried as a prefix.
        let filter = |param: &'static str, paths: &[&str]| {
            Some((param, paths.iter().map(|p| p.to_string()).collect()))
        };
        assert_eq!(
            set.run_queries(),
            [
                ("script", filter("script_path_exact", &["f/a/x"])),
                ("script", filter("script_path_start", &["f/ops/"])),
                ("flow", filter("script_path_exact", &["f/a/sync"])),
            ]
        );
    }
}

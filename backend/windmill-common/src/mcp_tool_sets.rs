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

/// What a token signed in to `name` acts with: the set as a selection of scripts and flows, with
/// no endpoint tools.
pub fn tool_set_scopes(name: &str, set: &McpToolSet) -> Vec<String> {
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
}

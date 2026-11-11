//! Workspace items that code names by path, and what it does with each.
//!
//! Assets describe the data a script touches; references describe the items whoever runs the
//! code needs permission on: the resources and variables it reads or writes, and the scripts
//! and flows it starts.

use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Resource,
    Variable,
    Script,
    Flow,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceAccess {
    Read,
    Write,
    /// Started through the run API, which reads the item as the caller.
    Run,
    /// Dispatched as a workflow-as-code task. The worker resolves it without the caller's
    /// permissions, so it needs no grant, but it still runs as the target's identity.
    Task,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemReference {
    pub kind: ItemKind,
    pub path: String,
    pub access: ReferenceAccess,
}

const PREFIXES: [(&str, ItemKind); 5] = [
    ("$res:", ItemKind::Resource),
    ("res://", ItemKind::Resource),
    ("$var:", ItemKind::Variable),
    ("$jsonvar:", ItemKind::Variable),
    ("var://", ItemKind::Variable),
];

impl ItemReference {
    /// `literal` as a reference when it names an item: a bare path, or a resource or variable
    /// path written with one of its prefixes.
    pub fn of(kind: ItemKind, access: ReferenceAccess, literal: &str) -> Option<Self> {
        let literal = literal.trim();
        let path = PREFIXES
            .iter()
            .find(|(prefix, k)| *k == kind && literal.starts_with(prefix))
            .map_or(literal, |(prefix, _)| &literal[prefix.len()..]);
        let path = match kind {
            // A resource named as an asset can carry `?table=…`: the item is what precedes it.
            ItemKind::Resource => path.split('?').next().unwrap_or(path),
            // The runtime parses a script or flow path with `RunnableRef::from_path`, which drops
            // one leading `/`.
            ItemKind::Script | ItemKind::Flow => path.strip_prefix('/').unwrap_or(path),
            ItemKind::Variable => path,
        };
        is_item_path(path).then(|| ItemReference { kind, path: path.to_string(), access })
    }

    /// A literal that names a resource or variable by its prefix (`$res:…`, `var://…`), as a read.
    pub fn of_prefixed(literal: &str) -> Option<Self> {
        let (_, kind) = PREFIXES
            .iter()
            .find(|(prefix, _)| literal.starts_with(prefix))?;
        Self::of(*kind, ReferenceAccess::Read, literal)
    }
}

/// Mirrors the `proper_id` CHECK on item paths: `u/`, `f/` or `g/` followed by at least two
/// segments of letters, digits, `_` and `-`.
pub fn is_item_path(path: &str) -> bool {
    let mut segments = path.split('/');
    if !matches!(segments.next(), Some("u" | "f" | "g")) || path.chars().count() > 255 {
        return false;
    }
    let mut count = 0;
    for segment in segments {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return false;
        }
        count += 1;
    }
    count >= 2
}

/// A call the lexical scan recognizes by name. `args` are the argument positions that can hold
/// the path, tried in order: an SDK whose argument order differs across languages lists both.
struct LexicalCall {
    names: &'static [&'static str],
    kind: ItemKind,
    access: ReferenceAccess,
    args: &'static [usize],
}

const SDK_CALLS: &[LexicalCall] = &[
    LexicalCall {
        names: &["getVariable", "get_variable", "GetVariable"],
        kind: ItemKind::Variable,
        access: ReferenceAccess::Read,
        args: &[0],
    },
    LexicalCall {
        names: &["setVariable", "set_variable", "SetVariable"],
        kind: ItemKind::Variable,
        access: ReferenceAccess::Write,
        args: &[0],
    },
    LexicalCall {
        names: &[
            "getResource",
            "get_resource",
            "GetResource",
            "databaseUrlFromResource",
            "denoS3LightClientSettings",
            "duckdbConnectionSettings",
            "polarsConnectionSettings",
            "duckdb_connection_settings",
            "polars_connection_settings",
            "boto3_connection_settings",
            "get_duckdb_connection_settings",
            "get_polars_connection_settings",
            "get_boto3_connection_settings",
            "request_interactive_slack_approval",
        ],
        kind: ItemKind::Resource,
        access: ReferenceAccess::Read,
        args: &[0],
    },
    LexicalCall {
        names: &["setResource", "set_resource", "SetResource"],
        kind: ItemKind::Resource,
        access: ReferenceAccess::Write,
        args: &[0, 1],
    },
    LexicalCall {
        names: &[
            "loadS3File",
            "loadS3FileStream",
            "load_s3_file",
            "load_s3_file_reader",
        ],
        kind: ItemKind::Resource,
        access: ReferenceAccess::Read,
        args: &[1],
    },
    LexicalCall {
        names: &["writeS3File", "write_s3_file"],
        kind: ItemKind::Resource,
        access: ReferenceAccess::Read,
        args: &[2],
    },
    LexicalCall {
        names: &[
            "runScript",
            "runScriptAsync",
            "runScriptByPath",
            "runScriptByPathAsync",
            "run_script",
            "run_script_async",
            "run_script_by_path",
            "run_script_by_path_async",
            "run_script_by_path_sync",
        ],
        kind: ItemKind::Script,
        access: ReferenceAccess::Run,
        args: &[0],
    },
    LexicalCall {
        names: &["runFlow", "runFlowAsync", "run_flow", "run_flow_async"],
        kind: ItemKind::Flow,
        access: ReferenceAccess::Run,
        args: &[0],
    },
    LexicalCall {
        names: &["taskScript", "task_script"],
        kind: ItemKind::Script,
        access: ReferenceAccess::Task,
        args: &[0],
    },
    LexicalCall {
        names: &["taskFlow", "task_flow"],
        kind: ItemKind::Flow,
        access: ReferenceAccess::Task,
        args: &[0],
    },
];

const INPUT_TRANSFORM_CALLS: &[LexicalCall] = &[
    LexicalCall {
        names: &["variable"],
        kind: ItemKind::Variable,
        access: ReferenceAccess::Read,
        args: &[0],
    },
    LexicalCall {
        names: &["resource"],
        kind: ItemKind::Resource,
        access: ReferenceAccess::Read,
        args: &[0],
    },
];

/// References in code no AST parser reads: languages without one, component markup, and code
/// that fails to parse. Finds SDK calls whose path argument is a quoted literal, and quoted
/// `$res:`/`$var:` literals. Lines that start as a comment are skipped.
pub fn scan_sdk_calls(code: &str) -> Vec<ItemReference> {
    scan(code, SDK_CALLS)
}

/// References in a flow input transform: its `variable('…')` and `resource('…')` calls, and
/// quoted `$res:`/`$var:` literals.
pub fn scan_input_transform(expr: &str) -> Vec<ItemReference> {
    scan(expr, INPUT_TRANSFORM_CALLS)
}

const COMMENT_STARTS: [&str; 6] = ["//", "#", "--", "/*", "*", "<!--"];
/// How far past a call's name its arguments are looked for, which keeps the scan linear.
const MAX_CALL_SPAN: usize = 4096;
/// How long a quoted literal can be and still be read as a path.
const MAX_LITERAL: usize = 512;

fn scan(code: &str, calls: &[LexicalCall]) -> Vec<ItemReference> {
    let bytes = code.as_bytes();
    let mut found = BTreeSet::new();
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        if line_start {
            line_start = false;
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            if COMMENT_STARTS.iter().any(|c| code[i..].starts_with(c)) {
                match code[i..].find('\n') {
                    Some(n) => {
                        i += n + 1;
                        line_start = true;
                        continue;
                    }
                    None => break,
                }
            }
            continue;
        }
        let b = bytes[i];
        if b == b'\n' {
            line_start = true;
            i += 1;
        } else if is_quote(b) {
            if let Some((literal, _)) = read_literal(code, i) {
                found.extend(ItemReference::of_prefixed(literal));
            }
            i += 1;
        } else if is_ident_start(b) && (i == 0 || !is_ident_char(bytes[i - 1])) {
            let start = i;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            let name = &code[start..i];
            if let Some(call) = calls.iter().find(|c| c.names.contains(&name)) {
                found.extend(read_call(code, i, call));
            }
        } else {
            i += 1;
        }
    }
    found.into_iter().collect()
}

/// The reference a call names, reading its arguments from just after its name.
fn read_call(code: &str, from: usize, call: &LexicalCall) -> Option<ItemReference> {
    let bytes = code.as_bytes();
    let limit = (from + MAX_CALL_SPAN).min(bytes.len());
    let mut i = skip_blank(bytes, from, limit);
    if bytes.get(i) != Some(&b'(') {
        return None;
    }
    i += 1;
    let last = *call.args.iter().max()?;
    for arg in 0..=last {
        i = skip_blank(bytes, i, limit);
        if i >= limit {
            return None;
        }
        let literal = if is_quote(bytes[i]) {
            read_literal(code, i)
        } else {
            None
        };
        i = match literal {
            Some((value, end)) => {
                let next = skip_blank(bytes, end, limit);
                let whole = matches!(bytes.get(next), Some(b',' | b')'));
                if whole && call.args.contains(&arg) {
                    if let Some(r) = ItemReference::of(call.kind, call.access, value) {
                        return Some(r);
                    }
                }
                next
            }
            None => skip_argument(bytes, i, limit),
        };
        if bytes.get(i) != Some(&b',') {
            return None;
        }
        i += 1;
    }
    None
}

/// A quoted literal starting at `at`, refused when it holds an escape, a line break or an
/// interpolation, since its text is then not the value the code uses.
fn read_literal(code: &str, at: usize) -> Option<(&str, usize)> {
    let bytes = code.as_bytes();
    let quote = bytes[at];
    let start = at + 1;
    let end = start
        + bytes[start..(start + MAX_LITERAL).min(bytes.len())]
            .iter()
            .position(|b| *b == quote)?;
    let value = &code[start..end];
    let refused =
        value.contains('\\') || value.contains('\n') || (quote == b'`' && value.contains("${"));
    (!refused).then_some((value, end + 1))
}

/// The position of the `,` or `)` that ends the argument starting at `from`.
fn skip_argument(bytes: &[u8], from: usize, limit: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < limit {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => return i,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => return i,
            q if is_quote(q) => {
                i += 1;
                while i < limit && bytes[i] != q {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    limit
}

fn skip_blank(bytes: &[u8], mut i: usize, limit: usize) -> usize {
    while i < limit && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn is_quote(b: u8) -> bool {
    matches!(b, b'"' | b'\'' | b'`')
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

#[cfg(test)]
mod tests {
    use super::*;
    use ItemKind::*;
    use ReferenceAccess::*;

    fn r(kind: ItemKind, path: &str, access: ReferenceAccess) -> ItemReference {
        ItemReference { kind, path: path.to_string(), access }
    }

    #[test]
    fn lexical_scan_reads_only_whole_literal_arguments() {
        let code = r#"
package inner

// wmill.GetVariable("u/admin/commented_out")
func main() {
	token, _ := wmill.GetVariable("f/ops/token")
	wmill.SetResource(map[string]any{"a": 1}, "f/ops/state")
	wmill.GetVariable(`f/ops/${name}`)
	wmill.GetResource("f/ops/" + name)
	run_script_by_path('f/ops/cleanup', args)
	other := "$res:f/ops/db"
}
"#;
        assert_eq!(
            scan_sdk_calls(code),
            vec![
                r(Resource, "f/ops/db", Read),
                r(Resource, "f/ops/state", Write),
                r(Variable, "f/ops/token", Read),
                r(Script, "f/ops/cleanup", Run),
            ]
        );
    }

    #[test]
    fn a_script_or_flow_path_may_start_with_one_slash() {
        assert_eq!(
            ItemReference::of(Script, Task, "/f/ops/cleanup"),
            Some(r(Script, "f/ops/cleanup", Task))
        );
        assert_eq!(ItemReference::of(Flow, Run, "//f/ops/nightly"), None);
    }
}

//! Workspace DuckDB macro libraries (`// macros` annotation).
//!
//! A macro-library script's body is `CREATE [OR REPLACE] [TEMP] MACRO`
//! statements plus plain setup (ATTACH/INSTALL/LOAD/SET/PRAGMA). At deploy the
//! macros are parsed into the `macro_definition` registry; at job time the
//! worker injects the (transitively) called ones into consumer scripts as
//! `CREATE OR REPLACE TEMP MACRO` blocks. Everything is stored and re-emitted
//! **verbatim** (params, body) — no AST round-trip — so any expression DuckDB
//! accepts survives unchanged.
//!
//! DuckDB bind-checks macro bodies at CREATE time (macro→macro and table
//! references alike), so injected definitions must be emitted in dependency
//! order — hence `topo_order_macros` — and after the consumer's ATTACHes.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::sql_materialize::{classify_block, split_statements, BlockClass};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMacro {
    /// Lowercased bare identifier (DuckDB identifiers are case-insensitive
    /// unquoted; qualified / quoted names are rejected at parse).
    pub name: String,
    /// Verbatim text inside the parameter parens (may be empty).
    pub params: String,
    /// Verbatim text after `AS [TABLE]`, without the trailing `;`.
    pub body: String,
    pub is_table: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibStatement {
    Macro(ParsedMacro),
    /// A non-macro statement allowed in a library: setup-class only
    /// (ATTACH/INSTALL/LOAD/SET/PRAGMA/USE/CREATE TEMP …). Re-emitted verbatim
    /// ahead of the macro definitions when the lib is injected via `// use`.
    Setup(String),
}

/// Statement text for injecting one macro into a consumer job. Always
/// TEMP (session-scoped — no catalog writes) and OR REPLACE (idempotent).
pub fn macro_create_statement(name: &str, params: &str, is_table: bool, body: &str) -> String {
    format!(
        "CREATE OR REPLACE TEMP MACRO {}({}) AS {}{};",
        name,
        params,
        if is_table { "TABLE " } else { "" },
        body
    )
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// Case-insensitive whole-word prefix strip (whitespace-bounded), returning the
// remainder with leading whitespace trimmed.
fn strip_kw<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    if s.len() >= kw.len() && s[..kw.len()].eq_ignore_ascii_case(kw) {
        let after = &s[kw.len()..];
        if after.is_empty() || after.starts_with(|c: char| c.is_whitespace()) {
            return Some(after.trim_start());
        }
    }
    None
}

/// Parse one (comment-free, `;`-less) statement as a CREATE MACRO. Returns
/// `Ok(None)` when the statement is not macro-shaped at all (caller decides
/// whether it is acceptable setup), `Err` when it is macro-shaped but invalid.
fn parse_create_macro(stmt: &str) -> Result<Option<ParsedMacro>, String> {
    let Some(mut rest) = strip_kw(stmt.trim(), "create") else {
        return Ok(None);
    };
    if let Some(r) = strip_kw(rest, "or") {
        rest = strip_kw(r, "replace").ok_or("expected REPLACE after CREATE OR")?;
    }
    if let Some(r) = strip_kw(rest, "temp").or_else(|| strip_kw(rest, "temporary")) {
        rest = r;
    }
    // `FUNCTION` is DuckDB's alias for `MACRO`.
    let Some(rest) = strip_kw(rest, "macro").or_else(|| strip_kw(rest, "function")) else {
        return Ok(None);
    };

    let name_end = rest
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(rest.len());
    let raw_name = &rest[..name_end];
    if raw_name.is_empty() {
        return Err("CREATE MACRO: missing macro name".to_string());
    }
    if raw_name.contains('.') || raw_name.contains('"') || !is_ident(raw_name) {
        return Err(format!(
            "macro name `{}` must be a plain unqualified identifier ([A-Za-z_][A-Za-z0-9_]*) in v1",
            raw_name
        ));
    }
    let name = raw_name.to_ascii_lowercase();

    let rest = rest[name_end..].trim_start();
    if !rest.starts_with('(') {
        return Err(format!(
            "macro `{}`: expected a parenthesized parameter list after the name",
            name
        ));
    }
    // Balanced-paren scan for the verbatim param list. The statement comes
    // from `split_statements` so comments are gone, but strings may contain
    // parens (e.g. a default value) — skip quoted spans.
    let bytes = rest.as_bytes();
    let mut depth = 0usize;
    let mut i = 0usize;
    let mut close = None;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            q @ (b'\'' | b'"') => {
                i += 1;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let Some(close) = close else {
        return Err(format!(
            "macro `{}`: unbalanced parameter parentheses",
            name
        ));
    };
    let params = rest[1..close].trim().to_string();

    let after_params = rest[close + 1..].trim_start();
    let Some(mut body) = strip_kw(after_params, "as") else {
        return Err(format!(
            "macro `{}`: expected AS after the parameter list",
            name
        ));
    };
    let is_table = match strip_kw(body, "table") {
        Some(r) => {
            body = r;
            true
        }
        None => false,
    };
    let body = body.trim().trim_end_matches(';').trim_end().to_string();
    if body.is_empty() {
        return Err(format!("macro `{}`: empty body", name));
    }
    Ok(Some(ParsedMacro { name, params, body, is_table }))
}

fn stmt_head(stmt: &str) -> String {
    stmt.split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Managed ATTACH forms (`ducklake://…`, `datatable://…`, resource URIs) are
/// rewritten by the worker's transform pass — which `// use`-injected lib
/// setup bypasses — so a library may only use plain, self-contained ATTACHes.
pub fn is_managed_attach(stmt: &str) -> bool {
    let s = stmt.trim();
    if strip_kw(s, "attach").is_none() {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    [
        "'ducklake:",
        "'datatable:",
        "'windmill:",
        "'$res:",
        "\"ducklake:",
        "\"datatable:",
        "\"windmill:",
        "\"$res:",
    ]
    .iter()
    .any(|p| lower.contains(p))
}

/// Parse a `// macros` library body. Statements are either macro definitions
/// or setup; anything else is an error (user-facing message).
pub fn parse_macro_library(sql: &str) -> Result<Vec<LibStatement>, String> {
    let mut out = Vec::new();
    for stmt in split_statements(sql) {
        match parse_create_macro(&stmt)? {
            Some(m) => out.push(LibStatement::Macro(m)),
            None => match classify_block(&stmt) {
                BlockClass::Setup => {
                    if is_managed_attach(&stmt) {
                        return Err(format!(
                            "a `// macros` library cannot use managed ATTACH forms \
                             (ducklake://, datatable://, resource URIs) in v1 — its setup is \
                             injected verbatim into consumers, bypassing the ATTACH rewrite: `{}`",
                            stmt_head(&stmt)
                        ));
                    }
                    out.push(LibStatement::Setup(stmt))
                }
                _ => {
                    return Err(format!(
                        "a `// macros` library may only contain CREATE [OR REPLACE] MACRO \
                         statements plus setup (ATTACH/INSTALL/LOAD/SET/PRAGMA); found: `{}`",
                        stmt_head(&stmt)
                    ))
                }
            },
        }
    }
    Ok(out)
}

/// Names from `names` that `sql` calls: an identifier token immediately
/// followed by `(` (after optional whitespace), not `.`-qualified. Scans the
/// comment-stripped statements; string-literal contents are skipped. Matching
/// is deliberately lexical — over-matching only injects an unused TEMP macro.
pub fn detect_macro_calls(sql: &str, names: &HashSet<String>) -> HashSet<String> {
    let mut found = HashSet::new();
    if names.is_empty() {
        return found;
    }
    for stmt in split_statements(sql) {
        let bytes = stmt.as_bytes();
        let n = bytes.len();
        let mut i = 0usize;
        let mut prev: Option<u8> = None;
        while i < n {
            let c = bytes[i];
            // skip quoted spans ('' escape for single quotes is handled by
            // the fact that the reopened quote just starts another skip)
            if c == b'\'' || c == b'"' {
                let q = c;
                i += 1;
                while i < n && bytes[i] != q {
                    i += 1;
                }
                i += 1;
                prev = Some(q);
                continue;
            }
            let is_ident_start = c.is_ascii_alphabetic() || c == b'_';
            let prev_blocks =
                matches!(prev, Some(p) if p == b'.' || p.is_ascii_alphanumeric() || p == b'_');
            if is_ident_start && !prev_blocks {
                let start = i;
                while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = stmt[start..i].to_ascii_lowercase();
                let mut j = i;
                while j < n && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < n && bytes[j] == b'(' && names.contains(&word) {
                    found.insert(word);
                }
                prev = Some(bytes[i - 1]);
                continue;
            }
            prev = Some(c);
            i += 1;
        }
    }
    found
}

/// Order `selected` macro names so every macro comes after the macros its
/// body calls (Kahn's algorithm, name-sorted ties for determinism). `defs`
/// maps each selected name to its body. Errors on a dependency cycle (only
/// reachable via cross-library deploy interleaving — DuckDB itself could
/// never have bound a cycle).
pub fn topo_order_macros(
    selected: &HashSet<String>,
    defs: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let all: HashSet<String> = selected.clone();
    // deps[m] = selected macros m's body calls; rev[d] = macros depending on d
    let mut deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut rev: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in selected {
        let body = defs
            .get(name)
            .ok_or_else(|| format!("macro `{}` has no definition", name))?;
        let mut called = detect_macro_calls(body, &all);
        called.remove(name); // ignore self-recursion (DuckDB rejects it at CREATE anyway)
        for d in &called {
            rev.entry(d.clone()).or_default().insert(name.clone());
        }
        deps.insert(name.clone(), called.into_iter().collect());
    }
    let mut ready: BTreeSet<String> = deps
        .iter()
        .filter(|(_, d)| d.is_empty())
        .map(|(n, _)| n.clone())
        .collect();
    let mut out = Vec::with_capacity(selected.len());
    while let Some(name) = ready.iter().next().cloned() {
        ready.remove(&name);
        out.push(name.clone());
        if let Some(dependents) = rev.get(&name) {
            for dep in dependents.clone() {
                let d = deps.get_mut(&dep).unwrap();
                d.remove(&name);
                if d.is_empty() {
                    ready.insert(dep);
                }
            }
        }
        deps.remove(&name);
    }
    if !deps.is_empty() {
        let cycle: Vec<String> = deps.keys().cloned().collect();
        return Err(format!(
            "macro dependency cycle involving: {}",
            cycle.join(", ")
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_scalar_macro() {
        let lib =
            parse_macro_library("CREATE MACRO surrogate_key(a, b) AS md5(concat_ws('||', a, b));")
                .unwrap();
        assert_eq!(
            lib,
            vec![LibStatement::Macro(ParsedMacro {
                name: "surrogate_key".into(),
                params: "a, b".into(),
                body: "md5(concat_ws('||', a, b))".into(),
                is_table: false,
            })]
        );
    }

    #[test]
    fn parses_table_macro_or_replace_temp_and_function_alias() {
        let lib = parse_macro_library(
            "CREATE OR REPLACE TEMP MACRO top_n(t_max) AS TABLE SELECT * FROM t WHERE x <= t_max;\n\
             create function dbl(a) as a * 2;",
        )
        .unwrap();
        match &lib[0] {
            LibStatement::Macro(m) => {
                assert_eq!(m.name, "top_n");
                assert!(m.is_table);
                assert_eq!(m.body, "SELECT * FROM t WHERE x <= t_max");
            }
            other => panic!("expected macro, got {:?}", other),
        }
        match &lib[1] {
            LibStatement::Macro(m) => {
                assert_eq!(m.name, "dbl");
                assert!(!m.is_table);
            }
            other => panic!("expected macro, got {:?}", other),
        }
    }

    #[test]
    fn parses_default_params_and_nested_parens() {
        let lib = parse_macro_library(
            "CREATE MACRO safe_div(a, b, fallback := (0)) AS CASE WHEN b = 0 THEN fallback ELSE a / b END;",
        )
        .unwrap();
        match &lib[0] {
            LibStatement::Macro(m) => {
                assert_eq!(m.params, "a, b, fallback := (0)");
                assert!(m.body.starts_with("CASE WHEN"));
            }
            other => panic!("expected macro, got {:?}", other),
        }
    }

    #[test]
    fn params_with_string_containing_paren() {
        let lib = parse_macro_library("CREATE MACRO f(sep := '(') AS concat(sep, 'x');").unwrap();
        match &lib[0] {
            LibStatement::Macro(m) => assert_eq!(m.params, "sep := '('"),
            other => panic!("expected macro, got {:?}", other),
        }
    }

    #[test]
    fn setup_statements_allowed_and_kept_in_order() {
        let lib =
            parse_macro_library("-- a comment\nATTACH 'x.duckdb' AS ext;\nCREATE MACRO m() AS 1;")
                .unwrap();
        assert_eq!(lib.len(), 2);
        assert!(matches!(&lib[0], LibStatement::Setup(s) if s.starts_with("ATTACH")));
        assert!(matches!(&lib[1], LibStatement::Macro(_)));
    }

    #[test]
    fn rejects_non_setup_statements() {
        let err = parse_macro_library("CREATE MACRO m() AS 1; SELECT 1;").unwrap_err();
        assert!(err.contains("may only contain"), "{err}");
        let err = parse_macro_library("CREATE TABLE t(x int);").unwrap_err();
        assert!(err.contains("may only contain"), "{err}");
    }

    #[test]
    fn rejects_managed_attach_setup() {
        let err =
            parse_macro_library("ATTACH 'ducklake://analytics' AS lake;\nCREATE MACRO m() AS 1;")
                .unwrap_err();
        assert!(err.contains("managed ATTACH"), "{err}");
    }

    #[test]
    fn rejects_qualified_and_quoted_names() {
        assert!(parse_macro_library("CREATE MACRO lake.m(a) AS a;")
            .unwrap_err()
            .contains("unqualified"));
        assert!(parse_macro_library("CREATE MACRO \"weird name\"(a) AS a;").is_err());
    }

    #[test]
    fn rejects_missing_params_or_body() {
        assert!(parse_macro_library("CREATE MACRO m AS 1;").is_err());
        assert!(parse_macro_library("CREATE MACRO m(a);").is_err());
        assert!(parse_macro_library("CREATE MACRO m(a) AS ;").is_err());
    }

    #[test]
    fn detect_basic_and_word_boundaries() {
        let ns = names(&["dbl", "avg_x"]);
        let found = detect_macro_calls("SELECT dbl(1), my_dbl(2), avg_x (3) FROM t", &ns);
        assert!(found.contains("dbl"));
        assert!(found.contains("avg_x")); // whitespace before paren ok
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn detect_skips_qualified_strings_and_comments() {
        let ns = names(&["dbl"]);
        assert!(detect_macro_calls("SELECT lake.dbl(1)", &ns).is_empty());
        assert!(detect_macro_calls("SELECT 'dbl(1)'", &ns).is_empty());
        assert!(detect_macro_calls("-- dbl(1)\nSELECT 1", &ns).is_empty());
        assert!(detect_macro_calls("SELECT dbl FROM t", &ns).is_empty()); // no call parens
    }

    #[test]
    fn detect_case_insensitive_and_table_macro_position() {
        let ns = names(&["top_n"]);
        assert!(!detect_macro_calls("SELECT * FROM TOP_N(3)", &ns).is_empty());
    }

    #[test]
    fn topo_chain_and_diamond() {
        let mut defs = BTreeMap::new();
        defs.insert("a".to_string(), "b(1) + c(2)".to_string());
        defs.insert("b".to_string(), "d(1)".to_string());
        defs.insert("c".to_string(), "d(2)".to_string());
        defs.insert("d".to_string(), "1".to_string());
        let sel = names(&["a", "b", "c", "d"]);
        let order = topo_order_macros(&sel, &defs).unwrap();
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("d") < pos("b"));
        assert!(pos("d") < pos("c"));
        assert!(pos("b") < pos("a"));
        assert!(pos("c") < pos("a"));
    }

    #[test]
    fn topo_cycle_errors() {
        let mut defs = BTreeMap::new();
        defs.insert("a".to_string(), "b(1)".to_string());
        defs.insert("b".to_string(), "a(1)".to_string());
        let err = topo_order_macros(&names(&["a", "b"]), &defs).unwrap_err();
        assert!(err.contains("cycle"), "{err}");
    }

    #[test]
    fn create_statement_roundtrip() {
        assert_eq!(
            macro_create_statement("m", "a, b := 1", true, "SELECT a + b"),
            "CREATE OR REPLACE TEMP MACRO m(a, b := 1) AS TABLE SELECT a + b;"
        );
    }

    #[test]
    fn builtin_lookup() {
        use crate::duckdb_builtins::is_duckdb_builtin;
        assert!(is_duckdb_builtin("concat"));
        assert!(is_duckdb_builtin("CONCAT"));
        assert!(is_duckdb_builtin("read_csv"));
        assert!(!is_duckdb_builtin("surrogate_key"));
    }
}

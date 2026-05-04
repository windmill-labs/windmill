#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;

use lazy_static::lazy_static;
#[cfg(not(target_arch = "wasm32"))]
use regex::{Match, Regex};
#[cfg(target_arch = "wasm32")]
use regex_lite::{Match, Regex};

use serde_json::json;

use std::{
    collections::{HashMap, HashSet},
    iter::Peekable,
    str::CharIndices,
};
pub use windmill_parser::{
    s3_mode_extension, Arg, MainArgSignature, ObjectType, S3ModeFormat, Typ,
};

pub const SANITIZED_ENUM_STR: &str = "__sanitized_enum__";
pub const SANITIZED_RAW_STRING_STR: &str = "__sanitized_raw_string__";

pub fn parse_mysql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_mysql_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_oracledb_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_oracledb_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_pgsql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let (sig, _) = parse_pgsql_sig_with_typed_schema(code)?;
    Ok(sig)
}

pub fn parse_pgsql_sig_with_typed_schema(code: &str) -> anyhow::Result<(MainArgSignature, bool)> {
    let parsed = parse_pg_file(&code)?;
    if let Some((args, typed_schema)) = parsed {
        Ok((
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args,
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            },
            typed_schema,
        ))
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_bigquery_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_bigquery_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_duckdb_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_duckdb_file(&code)?;
    if let Some(args) = parsed {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_snowflake_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_snowflake_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_mssql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_mssql_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            auto_kind: None,
            has_preprocessor: None,
            ..Default::default()
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_db_resource(code: &str) -> Option<String> {
    let cap = RE_DB.captures(code);
    cap.map(|x| x.get(1).map(|x| x.as_str().to_string()).unwrap())
}

pub struct S3ModeArgs {
    pub prefix: Option<String>,
    pub storage: Option<String>,
    pub format: S3ModeFormat,
}
pub fn parse_s3_mode(code: &str) -> anyhow::Result<Option<S3ModeArgs>> {
    let cap = match RE_S3_MODE.captures(code) {
        Some(x) => x,
        None => return Ok(None),
    };
    let args_str = cap
        .get(1)
        .map(|x| x.as_str().to_string())
        .unwrap_or_default();

    let mut prefix = None;
    let mut storage = None;
    let mut format = S3ModeFormat::Json;

    for kv in args_str.split(' ').map(|kv| kv.trim()) {
        if kv.is_empty() {
            continue;
        }
        let mut it = kv.split('=');
        let (Some(key), Some(value)) = (it.next(), it.next()) else {
            return Err(anyhow!("Invalid S3 mode argument: {}", kv));
        };
        match (key.trim(), value.trim()) {
            ("prefix", _) => prefix = Some(value.to_string()),
            ("storage", _) => storage = Some(value.to_string()),
            ("format", "json") => format = S3ModeFormat::Json,
            ("format", "parquet") => format = S3ModeFormat::Parquet,
            ("format", "csv") => format = S3ModeFormat::Csv,
            ("format", format) => return Err(anyhow!("Invalid S3 mode format: {}", format)),
            (_, _) => return Err(anyhow!("Invalid S3 mode argument: {}", kv)),
        }
    }

    Ok(Some(S3ModeArgs { prefix, storage, format }))
}

pub fn parse_sql_blocks(code: &str, track_dollar_quotes: bool) -> Vec<&str> {
    let mut blocks = vec![];
    let mut last_idx = 0;

    run_on_sql_statement_matches(
        code,
        track_dollar_quotes,
        |char, _| char == ';',
        |idx, _| {
            blocks.push(&code[last_idx..=idx]);
            last_idx = idx + 1;
        },
    );
    if last_idx < code.len() {
        let last_block = &code[last_idx..];
        if RE_NONEMPTY_SQL_BLOCK.is_match(last_block) {
            blocks.push(last_block);
        }
    }
    blocks
}

lazy_static::lazy_static! {
    static ref RE_CODE_PGSQL: Regex = Regex::new(r#"(?m)\$(\d+)(?:::(\w+(?:\[\])?))?"#).unwrap();

    static ref RE_NONEMPTY_SQL_BLOCK: Regex = Regex::new(r#"(?m)^\s*[^\s](?:[^-]|$)"#).unwrap();

    static ref RE_DB: Regex = Regex::new(r#"(?m)^-- database (\S+) *(?:\r|\n|$)"#).unwrap();
    static ref RE_S3_MODE: Regex = Regex::new(r#"(?m)^-- s3( (.+))? *(?:\r|\n|$)"#).unwrap();

    // -- $1 name (type) = default
    static ref RE_ARG_MYSQL: Regex = Regex::new(r#"(?m)^-- \? (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();
    pub static ref RE_ARG_MYSQL_NAMED: Regex = Regex::new(r#"(?m)^-- :([a-z_][a-z0-9_]*) \((\w+(?:\([\w, ]+\))?)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    static ref RE_ARG_PGSQL: Regex = Regex::new(r#"(?m)^-- \$(\d+) (\w+)(?: \(([A-Za-z0-9_\[\]]+)\))?(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    // -- @name (type) = default
    static ref RE_ARG_BIGQUERY: Regex = Regex::new(r#"(?m)^-- @(\w+) \((\w+(?:\[\])?)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    // -- $name (type) = default
    static ref RE_ARG_DUCKDB: Regex = Regex::new(r#"(?m)^-- \$(\w+) \(([A-Za-z0-9_\[\]]+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    static ref RE_ARG_SNOWFLAKE: Regex = Regex::new(r#"(?m)^-- \? (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();


    static ref RE_ARG_MSSQL: Regex = Regex::new(r#"(?m)^-- @(?:P|p)\d+ (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    // used for `unsafe` sql interpolation
    // -- %%name%% (type) = default
    static ref RE_ARG_SQL_INTERPOLATION: Regex = Regex::new(r#"(?m)^--\s*%%([a-z_][a-z0-9_]*)%%[ \t]*([\w][\w \t\/]*)?(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();
}

fn parsed_default(parsed_typ: &Typ, default: String) -> Option<serde_json::Value> {
    match parsed_typ {
        _ if default.to_lowercase() == "null" => None,
        Typ::Int => default.parse::<i64>().ok().map(|x| json!(x)),
        Typ::Float => default.parse::<f64>().ok().map(|x| json!(x)),
        Typ::Bool => default.parse::<bool>().ok().map(|x| json!(x)),
        Typ::Str(_) if default.len() >= 2 && default.starts_with("'") && default.ends_with("'") => {
            Some(json!(&default[1..default.len() - 1]))
        }
        _ => Some(json!(default)),
    }
}

fn parse_oracledb_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    let mut using_named_args = false;
    for cap in RE_ARG_MYSQL_NAMED.captures_iter(code) {
        using_named_args = true;
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_oracledb_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));
        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    if !using_named_args {
        // backwards compatibility
        for cap in RE_ARG_MYSQL.captures_iter(code) {
            let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
            let typ = cap
                .get(2)
                .map(|x| x.as_str().to_string().to_lowercase())
                .unwrap();
            let default = cap.get(3).map(|x| x.as_str().to_string());
            let has_default = default.is_some();
            let parsed_typ = parse_oracledb_typ(typ.as_str());

            let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

            args.push(Arg {
                name,
                typ: parsed_typ,
                default: parsed_default,
                otyp: Some(typ),
                has_default,
                oidx: None,
                otyp_inferred: false,
            });
        }
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

fn parse_sql_sanitized_interpolation(code: &str) -> Vec<Arg> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_SQL_INTERPOLATION.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap.get(2).map(|x| x.as_str());
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let (parsed_typ, otyp) = parse_unsafe_typ(typ);

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));
        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(otyp.to_string()),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    args
}

fn parse_mysql_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    let mut using_named_args = false;
    for cap in RE_ARG_MYSQL_NAMED.captures_iter(code) {
        using_named_args = true;
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_mysql_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));
        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    if !using_named_args {
        // backwards compatibility
        for cap in RE_ARG_MYSQL.captures_iter(code) {
            let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
            let typ = cap
                .get(2)
                .map(|x| x.as_str().to_string().to_lowercase())
                .unwrap();
            let default = cap.get(3).map(|x| x.as_str().to_string());
            let has_default = default.is_some();
            let parsed_typ = parse_mysql_typ(typ.as_str());

            let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

            args.push(Arg {
                name,
                typ: parsed_typ,
                default: parsed_default,
                otyp: Some(typ),
                has_default,
                oidx: None,
                otyp_inferred: false,
            });
        }
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

enum ParserState {
    Normal,
    InSingleQuote,
    InDoubleQuote,
    InSingleLineComment,
    InMultiLineComment,
    // Stores the full `$tag$` delimiter (including both `$`s). On re-encountering
    // the same delimiter we return to `Normal`. An empty tag yields `$$`.
    InDollarQuote(String),
}

// If `code[idx..]` starts with a PostgreSQL dollar-quote delimiter (`$$` or
// `$tag$`), returns the delimiter's byte length. The tag follows identifier
// rules (first char letter/underscore, subsequent letters/digits/underscores),
// which naturally rejects placeholder syntax like `$1` or `$2::int`.
fn parse_dollar_quote_delimiter(code: &str, idx: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    if bytes.get(idx) != Some(&b'$') {
        return None;
    }
    let mut i = idx + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'$' {
            return Some(i + 1 - idx);
        }
        let is_first = i == idx + 1;
        let ok = if is_first {
            b.is_ascii_alphabetic() || b == b'_'
        } else {
            b.is_ascii_alphanumeric() || b == b'_'
        };
        if !ok {
            return None;
        }
        i += 1;
    }
    None
}

fn advance_past<I: Iterator<Item = (usize, char)>>(chars: &mut Peekable<I>, target: usize) {
    while chars.peek().map_or(false, |&(i, _)| i < target) {
        chars.next();
    }
}

fn run_on_sql_statement_matches<
    F1: FnMut(char, &mut Peekable<CharIndices>) -> bool,
    F2: FnMut(usize, &mut Peekable<CharIndices>) -> (),
>(
    code: &str,
    track_dollar_quotes: bool,
    mut cond: F1,
    mut case: F2,
) {
    let mut chars = code.char_indices().peekable();
    let mut state = ParserState::Normal;
    while let Some((idx, char)) = chars.next() {
        match (&state, char) {
            (ParserState::Normal, '\'') => {
                state = ParserState::InSingleQuote;
            }
            (ParserState::Normal, '"') => {
                state = ParserState::InDoubleQuote;
            }
            (ParserState::Normal, '-')
                if chars.peek().is_some_and(|&(_, next_char)| next_char == '-') =>
            {
                state = ParserState::InSingleLineComment;
            }
            (ParserState::Normal, '/')
                if chars.peek().is_some_and(|&(_, next_char)| next_char == '*') =>
            {
                state = ParserState::InMultiLineComment;
            }
            (ParserState::Normal, '$') if track_dollar_quotes => {
                if let Some(delim_len) = parse_dollar_quote_delimiter(code, idx) {
                    let delim_end = idx + delim_len;
                    let delim = code[idx..delim_end].to_string();
                    advance_past(&mut chars, delim_end);
                    state = ParserState::InDollarQuote(delim);
                } else if cond(char, &mut chars) {
                    case(idx, &mut chars);
                }
            }
            (ParserState::Normal, _) if cond(char, &mut chars) => {
                case(idx, &mut chars);
            }
            (ParserState::InSingleQuote, '\'') => {
                if chars
                    .peek()
                    .is_some_and(|&(_, next_char)| next_char == '\'')
                {
                    chars.next(); // skip the escaped single quote
                } else {
                    state = ParserState::Normal;
                }
            }
            (ParserState::InDoubleQuote, '"') => {
                if chars.peek().is_some_and(|&(_, next_char)| next_char == '"') {
                    chars.next(); // skip the escaped single quote
                } else {
                    state = ParserState::Normal;
                }
            }
            (ParserState::InSingleLineComment, '\n') => {
                state = ParserState::Normal;
            }
            (ParserState::InMultiLineComment, '*')
                if chars.peek().is_some_and(|&(_, next_char)| next_char == '/') =>
            {
                state = ParserState::Normal;
            }
            (ParserState::InDollarQuote(delim), '$') => {
                if code[idx..].starts_with(delim.as_str()) {
                    let target = idx + delim.len();
                    advance_past(&mut chars, target);
                    state = ParserState::Normal;
                }
            }
            _ => {}
        }
    }
}

pub fn parse_pg_statement_arg_indices(code: &str) -> HashSet<i32> {
    parse_pg_statement_arg_positions(code)
        .into_iter()
        .map(|(idx, _)| idx)
        .collect()
}

/// Like `parse_pg_statement_arg_indices`, but also returns the byte range of
/// each placeholder occurrence in `code` (excluding `$`, including the digits).
/// The same string-/comment-/dollar-quote-aware tokenizer is used, so
/// occurrences inside string literals and comments are correctly skipped —
/// this is what callers need to renumber `$N → $M` without mangling literal
/// SQL bytes that happen to match the `$\d+` pattern.
///
/// The returned vec is in source order. Each entry is `(idx, range)` where
/// `idx` is the parameter number and `range` covers the `$N` digits (i.e.
/// `code[range.start - 1 .. range.end]` is the full `$N` token, and
/// `code[range]` is just the digits).
pub fn parse_pg_statement_arg_positions(code: &str) -> Vec<(i32, std::ops::Range<usize>)> {
    let mut positions = Vec::new();
    run_on_sql_statement_matches(
        code,
        true,
        |char, chars| {
            char == '$'
                && chars
                    .peek()
                    .is_some_and(|&(_, next_char)| next_char.is_ascii_digit())
        },
        |_, chars| {
            let start = chars.peek().map(|&(i, _)| i).unwrap_or(0);
            let mut arg_idx = String::new();
            let mut end = start;
            while let Some(&(i, char)) = chars.peek() {
                if char.is_ascii_digit() {
                    arg_idx.push(char);
                    end = i + char.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(arg_idx) = arg_idx.parse::<i32>() {
                positions.push((arg_idx, start..end));
            }
        },
    );
    positions
}

fn parse_pg_file(code: &str) -> anyhow::Result<Option<(Vec<Arg>, bool)>> {
    let mut args = vec![];

    // Track which args have explicit types in declaration comments
    let mut explicitly_typed_args: HashSet<i32> = HashSet::new();

    // First pass: collect args from declaration comments (-- $1 argName (type))
    for cap in RE_ARG_PGSQL.captures_iter(code) {
        let idx = cap
            .get(1)
            .and_then(|x| x.as_str().parse::<i32>().ok())
            .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?;

        let name = cap.get(2).map(|x| x.as_str().to_string()).unwrap();
        let explicit_type = cap.get(3).map(|x| x.as_str().to_string().to_lowercase());
        let default = cap.get(4).map(|x| x.as_str().to_string());
        let has_default = default.is_some();

        if let Some(typ) = explicit_type {
            // If explicitly typed, use that type and don't infer from usage
            explicitly_typed_args.insert(idx);
            let parsed_typ = parse_pg_typ(typ.as_str());
            let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

            args.push(Arg {
                name,
                typ: parsed_typ,
                default: parsed_default,
                otyp: Some(typ),
                has_default,
                oidx: Some(idx),
                otyp_inferred: false,
            });
        }
    }

    // Second pass: infer types from usage for non-explicitly-typed args.
    // We track whether each entry came from an inline `$N::TYPE` cast or from
    // the parser's "text" fallback, so the executor can later distinguish
    // "user committed to text" from "no info, use a placeholder".
    let mut hm: HashMap<i32, (String, bool)> = HashMap::new();
    for cap in RE_CODE_PGSQL.captures_iter(code) {
        let idx = cap
            .get(1)
            .and_then(|x| x.as_str().parse::<i32>().ok())
            .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?;

        // Skip if this arg was explicitly typed in declaration
        if explicitly_typed_args.contains(&idx) {
            continue;
        }

        let cast = cap
            .get(2)
            .map(|cap| transform_types_with_spaces(&cap, &code));
        let inferred_default = cast.is_none();
        let typ: std::borrow::Cow<str> = cast.unwrap_or(std::borrow::Cow::Borrowed("text"));
        // Prefer an explicit cast over a previously seen default — once we
        // have any inline cast for the index, lock it in.
        match hm.get(&idx) {
            Some((_, false)) => {} // already locked from explicit cast
            _ => {
                hm.insert(idx, (typ.into_owned(), inferred_default));
            }
        }
    }

    // Add inferred args
    for (i, (v, inferred)) in hm.iter() {
        let typ = v.to_lowercase();
        args.push(Arg {
            name: format!("${}", i),
            typ: parse_pg_typ(typ.as_str()),
            default: None,
            otyp: Some(typ),
            has_default: false,
            oidx: Some(*i),
            otyp_inferred: *inferred,
        });
    }

    // Sort by index
    args.sort_by(|a, b| a.oidx.unwrap().cmp(&b.oidx.unwrap()));

    // Third pass: update names and defaults for inferred args
    for cap in RE_ARG_PGSQL.captures_iter(code) {
        let i = cap
            .get(1)
            .and_then(|x| x.as_str().parse::<i32>().ok())
            .map(|x| x);

        // Skip explicitly typed args (already handled)
        if i.is_some_and(|idx| explicitly_typed_args.contains(&idx)) {
            continue;
        }

        if let Some(arg_pos) = args
            .iter()
            .position(|x| i.is_some_and(|i| x.oidx.unwrap() == i))
        {
            let name = cap.get(2).map(|x| x.as_str().to_string()).unwrap();
            let default = cap.get(4).map(|x| x.as_str().to_string());
            let has_default = default.is_some();
            let oarg = args[arg_pos].clone();
            let parsed_default = default.and_then(|x| parsed_default(&oarg.typ, x));

            args[arg_pos] = Arg {
                name,
                typ: oarg.typ,
                default: parsed_default,
                otyp: oarg.otyp,
                has_default,
                oidx: oarg.oidx,
                otyp_inferred: oarg.otyp_inferred,
            };
        }
    }

    let typed_schema = !explicitly_typed_args.is_empty();

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some((args, typed_schema)))
}

// The regex doesn't parse types with space such as "character varying"
// So we look for them manually and replace them with their shorter counterpart.
// Returns `Cow::Borrowed` for the trivial case (the regex's own match) and
// `Cow::Owned` when we need to alias a multi-word type and/or append a `[]`
// suffix that the regex's `\w+` capture didn't pick up.
fn transform_types_with_spaces<'a>(cap: &Match<'a>, code: &str) -> std::borrow::Cow<'a, str> {
    use std::borrow::Cow;
    lazy_static! {
        static ref TYPES: [(&'static str, &'static str); 6] = [
            ("character varying", "varchar"),
            ("double precision", "double"),
            ("time with time zone", "timetz"),
            ("time without time zone", "time"),
            ("timestamp with time zone", "timestamptz"),
            ("timestamp without time zone", "timestamp"),
        ];
    }
    let typ = &code[cap.start()..];
    for (long_type, alias) in TYPES.iter() {
        let mut rest = typ;
        let mut found_mismatch = false;
        for token in long_type.split(' ') {
            if rest.len() < token.len() || !rest[..token.len()].eq_ignore_ascii_case(token) {
                found_mismatch = true;
                break;
            }
            rest = rest[token.len()..].trim_start();
        }
        if !found_mismatch {
            // The regex captured only the first word (`\w+`), so its `[]`
            // detection in `(?:\[\])?` matched against the wrong position
            // and is empty for multi-word types. Re-check the trailing
            // bytes after the multi-word match: if they start with `[]`,
            // append the array suffix to the alias so the dispatch routes
            // through `convert_vec_val` instead of binding as JSONB.
            let with_suffix = rest.starts_with("[]");
            return if with_suffix {
                Cow::Owned(format!("{alias}[]"))
            } else {
                Cow::Borrowed(*alias)
            };
        }
    }
    Cow::Borrowed(cap.as_str())
}

pub fn parse_sql_statement_named_params(code: &str, prefix: char) -> HashSet<String> {
    let mut arg_names = HashSet::new();
    run_on_sql_statement_matches(
        code,
        false,
        |char, chars| {
            char == prefix
                && chars
                    .peek()
                    .is_some_and(|&(_, next_char)| next_char.is_alphanumeric() || next_char == '_')
        },
        |_, chars| {
            let mut arg_name = String::new();
            while let Some(&(_, char)) = chars.peek() {
                if char.is_alphanumeric() || char == '_' {
                    arg_name.push(char);
                    chars.next();
                } else {
                    break;
                }
            }
            arg_names.insert(arg_name);
        },
    );
    arg_names
}

fn parse_bigquery_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_BIGQUERY.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_bigquery_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

fn parse_duckdb_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_DUCKDB.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_duckdb_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

fn parse_snowflake_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_SNOWFLAKE.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_snowflake_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

fn parse_mssql_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_MSSQL.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap
            .get(2)
            .map(|x| x.as_str().to_string().to_lowercase())
            .unwrap();
        let default = cap.get(3).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_mssql_typ(typ.as_str());

        let parsed_default = default.and_then(|x| parsed_default(&parsed_typ, x));

        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
            oidx: None,
            otyp_inferred: false,
        });
    }

    args.append(&mut parse_sql_sanitized_interpolation(code));
    Ok(Some(args))
}

fn parse_unsafe_typ(typ: Option<&str>) -> (Typ, &'static str) {
    match typ {
        Some(s) => {
            let variants = s
                .split("/")
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();

            (Typ::Str(Some(variants)), SANITIZED_ENUM_STR)
        }
        None => (Typ::Str(None), SANITIZED_RAW_STRING_STR),
    }
}

pub fn parse_mysql_typ(typ: &str) -> Typ {
    match typ {
        "varchar" | "char" | "binary" | "varbinary" | "blob" | "text" | "enum" | "set" => {
            Typ::Str(None)
        }
        "int" | "uint" | "integer" => Typ::Int,
        "bool" | "bit" => Typ::Bool,
        "double precision" | "float" | "real" | "dec" | "fixed" => Typ::Float,
        "date" | "datetime" | "timestamp" | "time" => Typ::Datetime,
        "s3object" => Typ::Resource("S3Object".to_string()),
        _ => Typ::Str(None),
    }
}

pub fn parse_oracledb_typ(typ: &str) -> Typ {
    match typ {
        "varchar" | "nvarchar" | "varchar2" | "char" | "nchar" | "nvarchar2" | "clob" | "blob"
        | "nclob" => Typ::Str(None),
        "integer" | "int" | "long" | "rowid" | "urowid" => Typ::Int,
        "bool" => Typ::Bool,
        "number" | "float" | "binary_float" | "binary_double" => Typ::Float,
        "date" | "datetime" | "timestamp" | "time" => Typ::Datetime,
        _ => Typ::Str(None),
    }
}

pub fn parse_pg_typ(typ: &str) -> Typ {
    if typ.ends_with("[]") {
        let base_typ = parse_pg_typ(typ.strip_suffix("[]").unwrap());
        Typ::List(Box::new(base_typ))
    } else {
        match typ {
            "varchar" | "character varying" => Typ::Str(None),
            "text" => Typ::Str(None),
            "int" | "integer" | "int4" => Typ::Int,
            "bigint" => Typ::Int,
            "bool" | "boolean" => Typ::Bool,
            "char" | "character" => Typ::Str(None),
            "json" | "jsonb" => Typ::Object(ObjectType::new(None, Some(vec![]))),
            "smallint" | "int2" => Typ::Int,
            "smallserial" | "serial2" => Typ::Int,
            "serial" | "serial4" => Typ::Int,
            "bigserial" | "serial8" => Typ::Int,
            "real" | "float4" => Typ::Float,
            "double" | "double precision" | "float8" => Typ::Float,
            "numeric" | "decimal" => Typ::Float,
            "oid" => Typ::Int,
            "date"
            | "time"
            | "timetz"
            | "time with time zone"
            | "time without time zone"
            | "timestamp"
            | "timestamptz"
            | "timestamp with time zone"
            | "timestamp without time zone" => Typ::Datetime,
            "bytea" => Typ::Bytes,
            "s3object" => Typ::Resource("S3Object".to_string()),
            _ => Typ::Str(None),
        }
    }
}

pub fn parse_bigquery_typ(typ: &str) -> Typ {
    if typ.ends_with("[]") {
        let base_typ = parse_bigquery_typ(typ.strip_suffix("[]").unwrap());
        Typ::List(Box::new(base_typ))
    } else {
        match typ {
            "string" => Typ::Str(None),
            "bytes" => Typ::Bytes,
            "json" => Typ::Object(ObjectType::new(None, Some(vec![]))),
            "timestamp" | "date" | "time" | "datetime" => Typ::Datetime,
            "integer" | "int64" => Typ::Int,
            "float" | "float64" | "numeric" | "bignumeric" => Typ::Float,
            "boolean" | "bool" => Typ::Bool,
            "s3object" => Typ::Resource("S3Object".to_string()),
            _ => Typ::Str(None),
        }
    }
}

pub fn parse_duckdb_typ(typ: &str) -> Typ {
    if typ.ends_with("[]") {
        let base_typ = parse_duckdb_typ(typ.strip_suffix("[]").unwrap());
        Typ::List(Box::new(base_typ))
    } else {
        match typ {
            "varchar" | "char" | "bpchar" | "text" | "string" => Typ::Str(None),
            "blob" | "bytea" | "binary" | "varbinary" | "bitstring" => Typ::Bytes,
            "boolean" | "bool" | "bit" | "logical" => Typ::Bool,
            "bigint" | "int8" | "long" | "integer" | "int4" | "int" | "smallint" | "int2"
            | "short" | "tinyint" | "int1" | "signed" | "ubigint" | "uhugeint" | "uinteger"
            | "usmallint" | "utinyint" => Typ::Int,
            "decimal" | "numeric" | "double" | "float8" | "float" | "float4" | "real" => Typ::Float,
            "date"
            | "time"
            | "timestamp with time zone"
            | "timestamptz"
            | "timestamp"
            | "datetime" => Typ::Datetime,
            "uuid" | "json" => Typ::Str(None),
            "interval" | "hugeint" => Typ::Str(None),
            "s3object" => Typ::Resource("S3Object".to_string()),
            _ => Typ::Str(None),
        }
    }
}

pub fn parse_snowflake_typ(typ: &str) -> Typ {
    match typ {
        "varchar" => Typ::Str(None),
        "binary" => Typ::Bytes,
        "date" | "time" | "timestamp" => Typ::Datetime,
        "int" => Typ::Int,
        "float" => Typ::Float,
        "boolean" => Typ::Bool,
        "s3object" => Typ::Resource("S3Object".to_string()),
        _ => Typ::Str(None),
    }
}

pub fn parse_mssql_typ(typ: &str) -> Typ {
    match typ {
        "char" | "text" | "varchar" | "nchar" | "nvarchar" | "ntext" => Typ::Str(None),
        "binary" | "varbinary" | "image" => Typ::Bytes,
        "date" | "datetime2" | "datetime" | "datetimeoffset" | "smalldatetime" | "time" => {
            Typ::Datetime
        }
        "bigint" | "int" | "tinyint" | "smallint" => Typ::Int,
        "float" | "real" | "numeric" | "decimal" => Typ::Float,
        "bit" => Typ::Bool,
        "s3object" => Typ::Resource("S3Object".to_string()),
        _ => Typ::Str(None),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_pgsql_sig() -> anyhow::Result<()> {
        let code = r#"
SELECT * FROM table WHERE token=$1::TEXT AND image=$2::BIGINT
"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "$1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(1),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "$2".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_mutli_sig() -> anyhow::Result<()> {
        let code = r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT $3::TEXT, $1::BIGINT;
SELECT $2::TEXT;
"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "param1".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: Some(1),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(3),
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_multi_2semi() -> anyhow::Result<()> {
        let code = r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT;
-- ;
SELECT $2::TEXT;
"#;
        assert_eq!(parse_sql_blocks(code, true).len(), 2);

        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_multi_1semi() -> anyhow::Result<()> {
        let code = r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT;
-- ;
SELECT $2::TEXT
"#;
        assert_eq!(
            parse_sql_blocks(code, true),
            vec![
                r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT;"#,
                r#"
-- ;
SELECT $2::TEXT
"#
            ]
        );

        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_single_1semi() -> anyhow::Result<()> {
        let code = r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT;
-- hey
"#;
        assert_eq!(
            parse_sql_blocks(code, true),
            vec![
                r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT;"#,
            ]
        );

        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_single_nosemi() -> anyhow::Result<()> {
        let code = r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT
"#;
        assert_eq!(
            parse_sql_blocks(code, true),
            vec![
                r#"
-- $1 param1
-- $2 param2
-- $3 param3
SELECT '--', ';' $3::TEXT, $1::BIGINT
"#
            ]
        );

        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_dollar_quoted_function() -> anyhow::Result<()> {
        let code = r#"CREATE OR REPLACE FUNCTION track_status_change()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status IS DISTINCT FROM NEW.status THEN
        INSERT INTO use_case_card_history (use_case_id, old_status, new_status)
        VALUES (NEW.id, OLD.status, NEW.status);
    END IF;
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;"#;
        assert_eq!(parse_sql_blocks(code, true), vec![code]);
        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_dollar_quoted_tagged() -> anyhow::Result<()> {
        let code = r#"CREATE FUNCTION f() RETURNS int AS $body$ BEGIN RETURN 1; END; $body$ LANGUAGE plpgsql;
SELECT 1;"#;
        let blocks = parse_sql_blocks(code, true);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("$body$"));
        assert!(blocks[0].contains("END;"));
        assert_eq!(blocks[1], "\nSELECT 1;");
        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_dollar_quote_does_not_match_placeholder() -> anyhow::Result<()> {
        // `$1`, `$2` must still be treated as placeholders, not dollar-quote openers.
        let code = r#"SELECT $1::TEXT, $2::BIGINT;
SELECT $1 FROM t;"#;
        assert_eq!(parse_sql_blocks(code, true).len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_pg_statement_arg_indices_inside_dollar_quote() -> anyhow::Result<()> {
        // `$1` inside a dollar-quoted body is part of the string literal, not a parameter.
        let code = r#"CREATE FUNCTION f() RETURNS int AS $$ SELECT $1 $$ LANGUAGE sql;
SELECT $2;"#;
        let indices = parse_pg_statement_arg_indices(code);
        assert!(!indices.contains(&1), "$1 inside $$...$$ should be ignored");
        assert!(
            indices.contains(&2),
            "$2 outside dollar-quote should be collected"
        );
        Ok(())
    }

    #[test]
    fn test_parse_pg_statement_arg_positions_skips_strings_and_comments() -> anyhow::Result<()> {
        // Each occurrence's byte range covers JUST the digits (after `$`).
        let code = "SELECT $5, $50";
        let positions = parse_pg_statement_arg_positions(code);
        let collected: Vec<(i32, &str)> = positions
            .iter()
            .map(|(idx, range)| (*idx, &code[range.clone()]))
            .collect();
        assert_eq!(collected, vec![(5, "5"), (50, "50")]);

        // String literals and comments must not produce positions — this is
        // what stops the do_postgresql_inner rewrite from mangling SQL like
        // `'price: $5'`.
        let code = "SELECT 'literal $5' AS lbl, $5 FROM t -- mention $5";
        let positions = parse_pg_statement_arg_positions(code);
        let positions_only: Vec<(i32, std::ops::Range<usize>)> = positions.clone();
        assert_eq!(
            positions_only.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![5],
            "only the real $5 between 'lbl,' and 'FROM' should be returned"
        );
        // The single returned position is the real placeholder (between
        // `lbl, ` and ` FROM`).
        let (idx, range) = &positions[0];
        assert_eq!(*idx, 5);
        // `code[range.start - 1 .. range.end]` should be the full `$5` token.
        assert_eq!(&code[range.start - 1..range.end], "$5");

        // Dollar-quoted blocks similarly skipped.
        let code = "SELECT $$body with $5 inside$$, $7 FROM t";
        let positions = parse_pg_statement_arg_positions(code);
        assert_eq!(
            positions.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![7],
            "$5 inside $$...$$ is part of the string"
        );

        // Repeat indices show up multiple times — caller can rewrite each.
        let code = "SELECT $1, $1, $2";
        let positions = parse_pg_statement_arg_positions(code);
        assert_eq!(
            positions.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![1, 1, 2]
        );
        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_non_pg_ignores_dollar_quotes() -> anyhow::Result<()> {
        // Non-Postgres dialects (MySQL/Oracle/BigQuery/Snowflake) pass `false`,
        // so a bare `$tag$...;...$tag$` sequence must still split on `;`.
        let code = "SELECT $foo$; SELECT 1;";
        assert_eq!(parse_sql_blocks(code, false).len(), 2);
        // With tracking on (PG), `$foo$` opens a dollar-quote that is never closed,
        // so the whole thing becomes a single block.
        assert_eq!(parse_sql_blocks(code, true).len(), 1);
        Ok(())
    }

    #[test]
    fn test_parse_sql_blocks_nested_tag_mismatch() -> anyhow::Result<()> {
        // An inner tag that doesn't match the outer one does not terminate the outer quote.
        let code = r#"CREATE FUNCTION f() RETURNS int AS $outer$ SELECT $inner$ x; y $inner$ ; $outer$ LANGUAGE sql;
SELECT 1;"#;
        let blocks = parse_sql_blocks(code, true);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("$outer$ LANGUAGE sql;"));
        Ok(())
    }

    #[test]
    fn test_parse_mysql_positional_sig() -> anyhow::Result<()> {
        let code = r#"
-- ? param1 (int) = 3
-- ? param2 (text)
SELECT ?, ?;
"#;
        assert_eq!(
            parse_mysql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "param1".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_mysql_sig() -> anyhow::Result<()> {
        let code = r#"
-- :param1 (int) = 3
-- :param2 (text)
-- :param_3 (text)
SELECT :param_3, :param1;
SELECT :param2;
"#;
        assert_eq!(
            parse_mysql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "param1".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param_3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_bigquery_sig() -> anyhow::Result<()> {
        let code = r#"
-- @token (string) = abc
-- @image (int64)
SELECT * FROM table WHERE token=@token AND image=@image;
SELECT @token;
"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_bigquery_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("string".to_string()),
                        name: "token".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("abc")),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("int64".to_string()),
                        name: "image".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_snowflake_sig() -> anyhow::Result<()> {
        let code = r#"
-- ? param1 (int) = 3
-- ? param2 (varchar)
SELECT ?, ?;
-- ? param3 (varchar)
SELECT ?;
"#;
        assert_eq!(
            parse_snowflake_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "param1".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    }
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_mssql_sig() -> anyhow::Result<()> {
        let code = r#"
-- @P1 param1 (int) = 3
-- @P2 param2 (varchar)
-- @P3 param3 (varchar)
SELECT @P3, @P1;
SELECT @P2;
"#;
        assert_eq!(
            parse_mssql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "param1".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }
    #[test]
    fn test_parse_oracledb_sig() -> anyhow::Result<()> {
        let code = r#"
-- :name1 (int) = 3
-- :name2 (text)
-- :name4 (text)
SELECT :name, :name2;
SELECT * FROM table_name WHERE thing = :name4;
"#;

        println!("{:#?}", parse_oracledb_sig(code)?);
        assert_eq!(
            parse_oracledb_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "name1".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "name2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "name4".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_explicit_type_at_declaration() -> anyhow::Result<()> {
        let code = r#"
-- $1 user_id (bigint)
-- $2 email
SELECT * FROM users WHERE id = $1 AND email = $2::text;
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "user_id".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: Some(1),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "email".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_explicit_type_with_default() -> anyhow::Result<()> {
        let code = r#"
-- $1 limit (integer) = 10
-- $2 offset (bigint) = 0
SELECT * FROM users LIMIT $1 OFFSET $2;
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("integer".to_string()),
                        name: "limit".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(10)),
                        has_default: true,
                        oidx: Some(1),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "offset".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(0)),
                        has_default: true,
                        oidx: Some(2),
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_mixed_explicit_and_inferred() -> anyhow::Result<()> {
        let code = r#"
-- $1 user_id (bigint)
-- $2 status
-- $3 created_at (timestamptz)
SELECT * FROM users
WHERE id = $1
  AND status = $2::text
  AND created_at > $3;
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "user_id".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: Some(1),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "status".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                        otyp_inferred: false,
                    },
                    Arg {
                        otyp: Some("timestamptz".to_string()),
                        name: "created_at".to_string(),
                        typ: Typ::Datetime,
                        default: None,
                        has_default: false,
                        oidx: Some(3),
                        otyp_inferred: false,
                    },
                ],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_explicit_type_array() -> anyhow::Result<()> {
        let code = r#"
-- $1 ids (bigint[])
SELECT * FROM users WHERE id = ANY($1);
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    otyp: Some("bigint[]".to_string()),
                    name: "ids".to_string(),
                    typ: Typ::List(Box::new(Typ::Int)),
                    default: None,
                    has_default: false,
                    oidx: Some(1),
                    otyp_inferred: false,
                },],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_explicit_type_does_not_infer_from_usage() -> anyhow::Result<()> {
        // Even though $1 is used as ::integer in the query,
        // the explicit type (text) should take precedence
        let code = r#"
-- $1 value (text)
SELECT $1::integer;
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    otyp: Some("text".to_string()),
                    name: "value".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: Some(1),
                    otyp_inferred: false,
                },],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_otyp_inferred_flag() -> anyhow::Result<()> {
        // Bare `$N` (no inline cast, no decl) should produce otyp = "text"
        // *and* otyp_inferred = true. This is the signal the PG executor
        // uses to decide whether the user committed to a text target.
        let code_bare = "SELECT $1, $2";
        let args = parse_pgsql_sig(code_bare)?.args;
        let map: HashMap<String, (Option<String>, bool)> = args
            .into_iter()
            .map(|a| (a.name, (a.otyp, a.otyp_inferred)))
            .collect();
        assert_eq!(
            map.get("$1").cloned(),
            Some((Some("text".to_string()), true)),
            "bare $1 → otyp_inferred true"
        );
        assert_eq!(
            map.get("$2").cloned(),
            Some((Some("text".to_string()), true)),
            "bare $2 → otyp_inferred true"
        );

        // Inline `$N::TYPE` cast → otyp_inferred = false (user committed).
        let args = parse_pgsql_sig("SELECT $1::int, $2::text")?.args;
        let map: HashMap<String, (Option<String>, bool)> = args
            .into_iter()
            .map(|a| (a.name, (a.otyp, a.otyp_inferred)))
            .collect();
        assert_eq!(
            map.get("$1").cloned(),
            Some((Some("int".to_string()), false))
        );
        assert_eq!(
            map.get("$2").cloned(),
            Some((Some("text".to_string()), false)),
            "explicit $2::text → otyp_inferred false (distinct from bare $2)"
        );

        // Declaration `-- $N name (TYPE)` → otyp_inferred = false (decl is
        // explicit by definition).
        let args = parse_pgsql_sig("-- $1 name (text)\nSELECT $1")?.args;
        assert_eq!(args[0].otyp.as_deref(), Some("text"));
        assert!(!args[0].otyp_inferred);

        // Mixed: $1 has decl, $2 is bare → flag differs per arg.
        let args = parse_pgsql_sig("-- $1 a (int)\nSELECT $1, $2")?.args;
        let map: HashMap<String, bool> = args
            .into_iter()
            .map(|a| (a.name, a.otyp_inferred))
            .collect();
        assert_eq!(map.get("a").copied(), Some(false), "$1 decl → not inferred");
        assert_eq!(map.get("$2").copied(), Some(true), "$2 bare → inferred");

        Ok(())
    }

    #[test]
    fn test_parse_s3object_arg_per_dialect() -> anyhow::Result<()> {
        // Confirms that `(s3object)` is recognised as a resource-typed arg in every
        // native SQL dialect that opts in (PG, MySQL, MSSQL, BigQuery, Snowflake).
        // The frontend uses `Typ::Resource("S3Object")` to render the S3 picker, and
        // the worker dispatches on `otyp == "s3object"` to fetch + bind the file.
        let s3 = || Typ::Resource("S3Object".to_string());

        assert_eq!(
            parse_pgsql_sig("-- $1 myfile (s3object)\nSELECT $1::jsonb;")?
                .args
                .into_iter()
                .map(|a| (a.name, a.typ, a.otyp))
                .collect::<Vec<_>>(),
            vec![("myfile".to_string(), s3(), Some("s3object".to_string()))]
        );

        assert_eq!(
            parse_mssql_sig("-- @P1 myfile (s3object)\nSELECT @P1;")?
                .args
                .into_iter()
                .map(|a| (a.name, a.typ, a.otyp))
                .collect::<Vec<_>>(),
            vec![("myfile".to_string(), s3(), Some("s3object".to_string()))]
        );

        assert_eq!(
            parse_mysql_sig("-- :myfile (s3object)\nSELECT :myfile;")?
                .args
                .into_iter()
                .map(|a| (a.name, a.typ, a.otyp))
                .collect::<Vec<_>>(),
            vec![("myfile".to_string(), s3(), Some("s3object".to_string()))]
        );

        assert_eq!(
            parse_bigquery_sig("-- @myfile (s3object)\nSELECT @myfile;")?
                .args
                .into_iter()
                .map(|a| (a.name, a.typ, a.otyp))
                .collect::<Vec<_>>(),
            vec![("myfile".to_string(), s3(), Some("s3object".to_string()))]
        );

        assert_eq!(
            parse_snowflake_sig("-- ? myfile (s3object)\nSELECT ?;")?
                .args
                .into_iter()
                .map(|a| (a.name, a.typ, a.otyp))
                .collect::<Vec<_>>(),
            vec![("myfile".to_string(), s3(), Some("s3object".to_string()))]
        );

        Ok(())
    }

    #[test]
    fn test_parse_pgsql_safe_interpolated_args() -> anyhow::Result<()> {
        // There was a bug where enum would be "angrycreative"/"bishop"/"test SELECT x"
        let code = r#"
-- %%table_name%% angrycreative/bishop/test
SELECT x
"#;
        assert_eq!(
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    otyp: Some("__sanitized_enum__".to_string()),
                    name: "table_name".to_string(),
                    typ: Typ::Str(Some(vec![
                        "angrycreative".to_string(),
                        "bishop".to_string(),
                        "test".to_string()
                    ])),
                    default: None,
                    has_default: false,
                    oidx: None,
                    otyp_inferred: false,
                },],
                auto_kind: None,
                has_preprocessor: None,
                ..Default::default()
            }
        );

        Ok(())
    }
}

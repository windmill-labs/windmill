#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;
use serde_json::json;

use std::{
    collections::{HashMap, HashSet},
    iter::Peekable,
    str::CharIndices,
};
pub use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_mysql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_mysql_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_pgsql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_pg_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_bigquery_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_bigquery_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_snowflake_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_snowflake_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_mssql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_mssql_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_db_resource(code: &str) -> Option<String> {
    let cap = RE_DB.captures(code);
    cap.map(|x| x.get(1).map(|x| x.as_str().to_string()).unwrap())
}

pub fn parse_sql_blocks(code: &str) -> Vec<&str> {
    let mut blocks = vec![];
    let mut last_idx = 0;

    run_on_sql_statement_matches(
        code,
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

    // -- $1 name (type) = default
    static ref RE_ARG_MYSQL: Regex = Regex::new(r#"(?m)^-- \? (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();
    pub static ref RE_ARG_MYSQL_NAMED: Regex = Regex::new(r#"(?m)^-- :([a-z_][a-z0-9_]*) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    static ref RE_ARG_PGSQL: Regex = Regex::new(r#"(?m)^-- \$(\d+) (\w+)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    // -- @name (type) = default
    static ref RE_ARG_BIGQUERY: Regex = Regex::new(r#"(?m)^-- @(\w+) \((\w+(?:\[\])?)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

    static ref RE_ARG_SNOWFLAKE: Regex = Regex::new(r#"(?m)^-- \? (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();


    static ref RE_ARG_MSSQL: Regex = Regex::new(r#"(?m)^-- @(?:P|p)\d+ (\w+) \((\w+)\)(?: ?\= ?(.+))? *(?:\r|\n|$)"#).unwrap();

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
            });
        }
    }

    Ok(Some(args))
}

enum ParserState {
    Normal,
    InSingleQuote,
    InDoubleQuote,
    InSingleLineComment,
    InMultiLineComment,
}

fn run_on_sql_statement_matches<
    F1: FnMut(char, &mut Peekable<CharIndices>) -> bool,
    F2: FnMut(usize, &mut Peekable<CharIndices>) -> (),
>(
    code: &str,
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
            _ => {}
        }
    }
}

pub fn parse_pg_statement_arg_indices(code: &str) -> HashSet<i32> {
    let mut arg_indices = HashSet::new();
    run_on_sql_statement_matches(
        code,
        |char, chars| {
            char == '$'
                && chars
                    .peek()
                    .is_some_and(|&(_, next_char)| next_char.is_ascii_digit())
        },
        |_, chars| {
            let mut arg_idx = String::new();
            while let Some(&(_, char)) = chars.peek() {
                if char.is_ascii_digit() {
                    arg_idx.push(char);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(arg_idx) = arg_idx.parse::<i32>() {
                arg_indices.insert(arg_idx);
            }
        },
    );
    arg_indices
}

fn parse_pg_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args = vec![];
    let mut hm: HashMap<i32, String> = HashMap::new();
    for cap in RE_CODE_PGSQL.captures_iter(code) {
        hm.insert(
            cap.get(1)
                .and_then(|x| x.as_str().parse::<i32>().ok())
                .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?,
            cap.get(2)
                .map(|x| x.as_str().to_string())
                .unwrap_or_else(|| "text".to_string()),
        );
    }
    for (i, v) in hm.iter() {
        let typ = v.to_lowercase();
        args.push(Arg {
            name: format!("${}", i),
            typ: parse_pg_typ(typ.as_str()),
            default: None,
            otyp: Some(typ),
            has_default: false,
            oidx: Some(*i),
        });
    }
    args.sort_by(|a, b| a.oidx.unwrap().cmp(&b.oidx.unwrap()));
    for cap in RE_ARG_PGSQL.captures_iter(code) {
        let i = cap
            .get(1)
            .and_then(|x| x.as_str().parse::<i32>().ok())
            .map(|x| x);

        if let Some(arg_pos) = args
            .iter()
            .position(|x| i.is_some_and(|i| x.oidx.unwrap() == i))
        {
            let name = cap.get(2).map(|x| x.as_str().to_string()).unwrap();
            let default = cap.get(3).map(|x| x.as_str().to_string());
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
            };
        }
    }

    Ok(Some(args))
}

pub fn parse_sql_statement_named_params(code: &str, prefix: char) -> HashSet<String> {
    let mut arg_names = HashSet::new();
    run_on_sql_statement_matches(
        code,
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
        });
    }

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
        });
    }

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
        });
    }

    Ok(Some(args))
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
            "json" | "jsonb" => Typ::Object(vec![]),
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
            "json" => Typ::Object(vec![]),
            "timestamp" | "date" | "time" | "datetime" => Typ::Datetime,
            "integer" | "int64" => Typ::Int,
            "float" | "float64" | "numeric" | "bignumeric" => Typ::Float,
            "boolean" | "bool" => Typ::Bool,
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
                    },
                    Arg {
                        otyp: Some("bigint".to_string()),
                        name: "$2".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                    },
                ],
                no_main_func: None
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
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(2),
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: Some(3),
                    },
                ],
                no_main_func: None
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
        assert_eq!(parse_sql_blocks(code).len(), 2);

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
            parse_sql_blocks(code),
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
            parse_sql_blocks(code),
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
            parse_sql_blocks(code),
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
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: None
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
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "param_3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: None
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
                    },
                    Arg {
                        otyp: Some("int64".to_string()),
                        name: "image".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: None
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
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    }
                ],
                no_main_func: None
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_mssql_sig() -> anyhow::Result<()> {
        let code = r#"
-- @p1 param1 (int) = 3
-- @p2 param2 (varchar)
-- @p3 param3 (varchar)
SELECT @p3, @p1;
SELECT @p2;
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
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        otyp: Some("varchar".to_string()),
                        name: "param3".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: None
            }
        );

        Ok(())
    }
}

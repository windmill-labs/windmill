#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

use anyhow::{anyhow, bail};
use nu_parser::lex;

use serde_json::{json, Value};
use windmill_parser::{Arg, MainArgSignature, ObjectType, Typ};

pub fn parse_nu_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    let (tokens, ..) = lex(code.as_bytes(), 0, &[], &[], true);
    let src = code.to_owned();
    #[derive(Debug)]
    enum LastToken {
        None,
        Def,
        Main,
        Args(String),
    }
    let mut last_token = LastToken::None;
    for token in tokens {
        let s = token.span;
        let cont = src.get(s.start..s.end).ok_or(anyhow!("Parsing error"))?;
        last_token = match last_token {
            LastToken::None if cont == "def" => LastToken::Def,
            LastToken::Def if cont == "main" => LastToken::Main,
            LastToken::Main => {
                LastToken::Args(cont.get(1..(cont.len() - 1)).unwrap_or("Error").to_owned())
            }
            LastToken::Args(_) => break,
            _ => LastToken::None,
        };
    }

    let LastToken::Args(args) = last_token else {
        bail!("Cannot find main function.");
    };

    let mut sig = MainArgSignature::default();
    sig.no_main_func = Some(false);

    let batches = args
        .lines()
        .filter_map(|el| {
            if el.trim_start().starts_with('#') {
                None
            } else {
                Some(
                    el.split(',')
                        .map(|el| el.trim())
                        .filter(|el| el != &"")
                        .collect::<Vec<&str>>(),
                )
            }
        })
        .flatten()
        .collect::<Vec<&str>>();

    let mut compensate_lookahead = 0;
    for (i, batch) in batches.iter().enumerate() {
        // parse_default can lookahead and if it does we need to compensate
        // otherwise we would try to parse data already parsed but not yielded by parse_default
        if compensate_lookahead > 0 {
            compensate_lookahead -= 1;
            continue;
        }

        let type_start = batch.find(":");
        let default_start = batch.find("=");

        let (name, typ, default) = match (type_start, default_start) {
            (None, None) => (batch.trim(), None, None),
            (None, Some(d)) => (
                batch
                    .get(0..d)
                    .ok_or(anyhow!("Cannot parse argument ident"))?
                    .trim(),
                None,
                Some(parse_default(
                    &batch
                        .get(d..)
                        .ok_or(anyhow!("Cannot parse default value for argument"))?,
                    &batches,
                    i,
                    &mut compensate_lookahead,
                )?),
            ),
            (Some(t), None) => (
                batch
                    .get(0..t)
                    .ok_or(anyhow!("Cannot parse argument ident"))?
                    .trim(),
                Some(parse_type(
                    &batch
                        .get(t..)
                        .ok_or(anyhow!("Cannot parse type of argument"))?,
                )?),
                None,
            ),
            (Some(t), Some(d)) => {
                if t < d {
                    (
                        batch
                            .get(0..t)
                            .ok_or(anyhow!("Cannot parse argument ident"))?
                            .trim(),
                        Some(parse_type(
                            &batch
                                .get(t..d)
                                .ok_or(anyhow!("Cannot parse type of argument"))?,
                        )?),
                        Some(parse_default(
                            &batch
                                .get(d..)
                                .ok_or(anyhow!("Cannot parse default value of argument"))?,
                            &batches,
                            i,
                            &mut compensate_lookahead,
                        )?),
                    )
                } else {
                    bail!("Parsing error `:` should be before `=`\nit likely means you are trying to set default value to record or table which is not supported at the moment.")
                }
            }
        };

        // Check if it is optional
        let optional = {
            let Some(element) = name.chars().last() else {
                bail!("Internal error, cannot check if argument is optional")
            };
            element == '?'
        };

        // Rest parameters are not supported
        if matches!(name.get(0..3), Some("...")) {
            bail!("Rest (...) parameters are not supported")
        }

        // Flags are not supported
        if matches!(name.get(0..2), Some("--")) {
            bail!("Flags are not supported")
        }

        sig.args.push(Arg {
            name: if optional {
                name.get(..name.len() - 1).unwrap_or("Error").to_owned()
            } else {
                name.to_owned()
            },
            typ: typ.unwrap_or(Typ::Unknown),
            otyp: None,
            has_default: default.is_some() || optional,
            default: default.or_else(|| if optional { Some(json!(null)) } else { None }),
            oidx: None,
        });
    }

    fn parse_type(content: &str) -> anyhow::Result<Typ> {
        let c = content.replace(":", "").trim().to_owned();
        let typ = match c.as_str() {
            "string" => Typ::Str(None),
            "int" => Typ::Int,
            "float" => Typ::Float,
            "number" => Typ::Float,
            "record" => Typ::Object(ObjectType::new(None, Some(vec![]))),
            "table" => Typ::List(Box::new(Typ::Object(ObjectType::new(None, Some(vec![]))))),
            "nothing" => Typ::Unknown,
            // TODO: needs additional work on literal parsing
            // "binary" => Typ::Bytes,
            "datetime" => Typ::Datetime,
            "any" => Typ::Unknown,
            "bool" => Typ::Bool,
            // Lists
            "list" | "list<any>" | "list<nothing>" => Typ::List(Box::new(Typ::Unknown)),
            "list<number>" => Typ::List(Box::new(Typ::Float)),
            "list<bool>" => Typ::List(Box::new(Typ::Bool)),
            "list<string>" => Typ::List(Box::new(Typ::Str(None))),
            // list<float/int> is not supported
            // Records and Tables
            // TODO: Support in V1?
            s if s.contains("record<") => {
                bail!("typed records are not supported, use `ident: record`")
            }
            s if s.contains("table<") => {
                bail!("typed tables are not supported, use `ident: table`")
            }
            s => bail!("{s} is not supported"),
        };
        Ok(typ)
    }
    fn parse_default(
        content: &str,
        ctx: &[&str],
        i: usize,
        skip: &mut usize,
    ) -> anyhow::Result<Value> {
        let mut c = content.replace("=", "").trim().to_owned();

        fn parse_object_literal(
            (open, close): (char, char),
            mut c_2: String,
            ctx: &[&str],
            i: usize,
            skip: &mut usize,
        ) -> anyhow::Result<String> {
            // It is list
            // if c.contains("[") {
            let mut closed = false;
            if c_2 != open.to_string() {
                // [a ~ , ~ ...
                //  Add ^
                c_2 += ",";
            }
            // else {
            // [
            // a  < Do not add ","
            // ...
            // }
            let remainder = &ctx
                .iter()
                .skip(i + 1)
                .map_while(|el| {
                    let el = el.trim();

                    if closed {
                        None
                    } else {
                        if el.chars().last() == Some(close) {
                            closed = true;
                        }
                        *skip += 1;
                        Some(el)
                    }
                })
                .collect::<Vec<&str>>()
                .join(",");

            if remainder.contains(&['{', '[']) {
                bail!("Nesting is not supported")
            }

            Ok((c_2 + remainder)
                // Remove trailing comma if there is any
                .replace(&format!(",{close}"), &close.to_string()))
        }
        // It is list
        if c.contains("[") {
            if c.chars().last() != Some(']') {
                c = parse_object_literal(('[', ']'), c.clone(), ctx, i, skip)?;
            }
        }
        // It is record
        if c.contains("{") {
            if c.chars().last() != Some('}') {
                c = parse_object_literal(('{', '}'), c.clone(), ctx, i, skip)?;
            }
        }
        Ok(serde_json::from_str(&c)?)
    }
    Ok(sig)
}

#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

use anyhow::bail;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
    PositionalArg, SyntaxShape,
};
use serde_json::{json, Map};
use windmill_parser::{Arg, MainArgSignature, ObjectProperty};

// TODO: Preprocessors
pub fn parse_nu_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    let engine_state = nu_cmd_lang::create_default_context();
    let mut set = StateWorkingSet::new(&engine_state);
    let block = { &nu_parser::parse(&mut set, None, code.as_bytes(), false) };

    let mut sig = MainArgSignature {
        no_main_func: Some(true),
        ..Default::default() //
    };

    for pipeline in &block.pipelines {
        for el in &pipeline.elements {
            if let Expr::Call(ref call) = el.expr.expr {
                let mut iter = call.positional_iter();
                match (iter.next(), iter.next()) {
                    (
                        Some(Expression { expr: Expr::String(fn_name), .. }),
                        Some(Expression { expr: Expr::Signature(nu_sig), .. }),
                    ) if fn_name == "main" => {
                        sig.no_main_func = Some(false);
                        let mut handle_arg =
                            |PositionalArg { name, desc: _, shape, var_id: _, default_value },
                             has_default|
                             -> anyhow::Result<()> {
                                let or = if has_default { Some(json!(null)) } else { None };
                                sig.args.push(Arg {
                                    name,
                                    typ: glue_types(shape, true)?,
                                    otyp: None,
                                    default: default_value
                                        .and_then(|val| parse_default_value(val).ok())
                                        .or(or),
                                    has_default,
                                    oidx: None,
                                });
                                Ok(())
                            };
                        for arg in nu_sig.required_positional.clone() {
                            handle_arg(arg, false)?;
                        }
                        for arg in nu_sig.optional_positional.clone() {
                            handle_arg(arg, true)?;
                        }
                        if let Some(arg) = nu_sig.rest_positional.clone() {
                            sig.star_args = true;
                            handle_arg(arg, false)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(sig)
}

fn parse_default_value(val: nu_protocol::Value) -> anyhow::Result<serde_json::Value> {
    use nu_protocol::Value::*;
    use serde_json::to_value;
    match val {
        Bool { val, .. } => to_value(val).map_err(anyhow::Error::from),
        Int { val, .. } => to_value(val).map_err(anyhow::Error::from),
        Float { val, .. } => to_value(val).map_err(anyhow::Error::from),
        String { val, .. } => to_value(val).map_err(anyhow::Error::from),
        Date { val, .. } => to_value(val).map_err(anyhow::Error::from),
        Record { val, .. } => Ok(serde_json::Value::Object(Map::from_iter({
            let mut fields = vec![];
            for (name, val) in <nu_protocol::Record as Clone>::clone(&val) {
                fields.push((name, parse_default_value(val)?));
            }
            fields.into_iter()
        }))),
        List { vals, .. } => {
            let mut json_values = vec![];
            for val in vals.into_iter() {
                json_values.push(parse_default_value(val)?);
            }
            Ok(serde_json::Value::Array(json_values))
        }
        Nothing { .. } => Ok(json!("null")),
        Binary { val, .. } => to_value(val).map_err(anyhow::Error::from),
        wc => Err(anyhow::anyhow!(
            "Unexpected Nu type node kind: {:?}. This type is not handeled by Windmill, please open an issue if this seems to be an error",
            wc,
        )),
    }
}

use nu_protocol::SyntaxShape::*;
use windmill_parser::Typ;
fn glue_types(shape: SyntaxShape, is_top_level: bool) -> anyhow::Result<Typ> {
    Ok(match shape {
        Any | Nothing => Typ::Unknown,
        Int => Typ::Int,
        Number => Typ::Float,
        Boolean => Typ::Bool,
        String => Typ::Str(None),

        Record(vec) => Typ::Object({
            let mut fields = vec![];
            for (key, shape) in vec.into_iter() {
                fields.push(
                    ObjectProperty {
                        key,
                        typ: Box::new(glue_types(shape, false)?),
                    }
                );
            }
            fields
        }),
        Table(vec) => Typ::List(Box::new(
            Typ::Object({
                let mut fields = vec![];
                for (key, shape) in vec.into_iter() {
                    fields.push(
                        ObjectProperty {
                            key,
                            typ: Box::new(glue_types(shape, false)?),
                        }
                    );
                }
                fields
            })
        )),

        List(syntax_shape) => Typ::List(Box::new(glue_types(*syntax_shape, false)?)),
        Binary if is_top_level => Typ::Bytes,
        DateTime if is_top_level => Typ::Datetime,
        Float if is_top_level => Typ::Float,

        Binary => bail!("nu: `binary` is only supported on top level."),
        DateTime => bail!("nu: `datetime` is only supported on top level. If you wish to use `datetime` in object you can use `string` and then convert to datetime manually `$in | into datetime` "),
        Float => bail!("nu: `float` is only supported on top level, use `number` or `int` instead."),
        t => bail!("nu: `{t}` is not handled by Windmill, please open ann issue if this seems to be an error"),
    })
}

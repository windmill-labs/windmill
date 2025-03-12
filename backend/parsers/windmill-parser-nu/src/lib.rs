#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::{anyhow, bail};
use nu_cmd_lang::Def;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{Command, EngineState, StateWorkingSet},
    PositionalArg, Span, SyntaxShape,
};
// use serde_json::{json, Map};
// use windmill_parser::{Arg, MainArgSignature, ObjectProperty};

// use tree_sitter::Node;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};

pub fn create_default_context() -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // Core
        bind_command! {
            Def,
        };

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    engine_state
}
// // TODO: Preprocessors?
pub fn parse_nu_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    // let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = create_default_context();

    let mut set = StateWorkingSet::new(&engine_state);
    // set.add_decl(Box::new(Command::))
    let file_id = set.add_file("source".to_owned(), code.as_bytes());
    // dbg!(set.get_signature(decl));
    // dbg!(String::from_utf8(dbg!(set
    //     .get_span_contents(Span { start: 0, end: 10 })
    //     .to_vec())));
    if let Some(id) = set.find_decl("def".as_bytes()) {
        let decl = set.get_decl(id);
        let signature = set.get_signature(decl);
        // let output = signature.get_output_type();
        dbg!(&signature.required_positional[1]);
    }

    dbg!(nu_parser::parser::parse_signature(
        &mut set,
        Span { start: 26, end: 38 }
    ));

    // lex_signature(input, span_offset, additional_whitespace, special_tokens, skip_comment)

    // let block = { &nu_parser::parse(&mut set, None, code.as_bytes(), false) };
    // dbg!(block);
    // nu_parser::parser::parse_call(&mut set, &[], Span::unknown());
    // parse_call(&mut set, &[]);

    // let mut parser = tree_sitter::Parser::new();
    // let language = tree_sitter_nu::LANGUAGE;
    // parser
    //     .set_language(&language.into())
    //     .map_err(|e| anyhow!("Error setting Nu as language: {e}"))?;

    // // Parse code
    // let tree = parser.parse(code, None).expect("Failed to parse code");
    // let root_node = tree.root_node();

    let mut sig = MainArgSignature::default();
    sig.no_main_func = Some(true);
    // Traverse the AST to find the Main method signature
    // let main_sig = find_main_signature(root_node, code);
    // let mut cursor = root_node.walk();
    // // let mut args = vec![];
    // for f in root_node.children(&mut cursor) {
    //     if dbg!(f.kind()) == "decl_def" {
    //         // dbg!(f.to_string());
    //         let mut curs = f.walk();
    //         let mut iter = f.children(&mut curs).skip(1);
    //         // for p in f.children(&mut f.walk()) {
    //         //     if dbg!(p.kind()) == "parameter_bracks" {
    //         // if let Expr::Call(ref call) = el.expr.expr {
    //         // let mut iter = call.positional_iter();
    //         match (iter.next(), iter.next()) {
    //             (Some(fn_name), Some(parameters))
    //                 if dbg!(fn_name.utf8_text(code.as_bytes()))
    //                     .map(|name| name == "main")
    //                     .unwrap_or(false) =>
    //             {
    //                 dbg!(parameters.to_string());
    //                 sig.no_main_func = Some(false);
    //                 // let mut handle_arg =
    //                 //     |PositionalArg { name, desc: _, shape, var_id: _, default_value },
    //                 //      has_default|
    //                 //      -> anyhow::Result<()> {
    //                 //         let or = if has_default { Some(json!(null)) } else { None };
    //                 //         sig.args.push(Arg {
    //                 //             name: name.clone(),
    //                 //             typ: glue_types(name, shape, true)?,
    //                 //             otyp: None,
    //                 //             default: default_value
    //                 //                 .and_then(|val| parse_default_value(val).ok())
    //                 //                 .or(or),
    //                 //             has_default,
    //                 //             oidx: None,
    //                 //         });
    //                 //         Ok(())
    //                 //     };
    //                 for p in parameters.children(&mut parameters.walk()) {
    //                     if p.kind() == "parameter" {
    //                         // dbg!(f.to_sexp());
    //                         let mut default = None;
    //                         for a in p.children(&mut p.walk()) {
    //                             if a.kind() == "param_long_flag" {
    //                                 bail!("Flags are not supported in main function signature")
    //                             }
    //                             if a.kind() == "param_rest" {
    //                                 bail!("Rest parameters are not supported in main function signature")
    //                             }
    //                             if dbg!(a.kind()) == "param_value" {
    //                                 dbg!(a.to_string());
    //                                 default =
    //                                     a.utf8_text(code.as_bytes()).ok().and_then(|content| {
    //                                         serde_json::from_str(dbg!(&content.replace("=", "").trim()))
    //                                             .ok()
    //                                     });

    //                                 dbg!(&default);

    //                                 // if let Some(node) = a.next_sibling() {
    //                                 // default = csharp_param_default_value(node, code);
    //                                 // }
    //                             } else if dbg!(a.kind()) == "param_opt" {
    //                                 default = Some(serde_json::Value::Null);
    //                             }
    //                         }
    //                         let (otyp, typ, name) = parse_nu_typ(p, code)?;
    //                         sig.args.push(Arg {
    //                             name,
    //                             otyp: None,
    //                             typ,
    //                             has_default: default.is_some(),
    //                             default,
    //                             oidx: None,
    //                         });

    //                         // dbg!(&args);
    //                     }
    //                 }
    //                 // for param in
    //                 //     parameters.children_by_field_name("parameter", &mut parameters.walk())
    //                 // {
    //                 //     for p_list_node in param_list.children(&mut param_list.walk()) {
    //                 //         if p_list_node.kind() == "parameter" {
    //                 //             let mut default = None;
    //                 //             for a in p_list_node.children(&mut p_list_node.walk()) {
    //                 //                 if a.kind() == "=" {
    //                 //                     if let Some(node) = a.next_sibling() {
    //                 //                         default = csharp_param_default_value(node, code);
    //                 //                     }
    //                 //                 }
    //                 //             }
    //                 //             let (otyp, typ, name) = parse_csharp_typ(p_list_node, code)?;
    //                 //             args.push(Arg {
    //                 //                 name,
    //                 //                 otyp,
    //                 //                 typ,
    //                 //                 default,
    //                 //                 has_default: false,
    //                 //                 oidx: None,
    //                 //             });
    //                 //         }
    //                 // }
    //                 // }
    //                 // for arg in nu_sig.required_positional.clone() {
    //                 //     handle_arg(arg, false)?;
    //                 // }
    //                 // for arg in nu_sig.optional_positional.clone() {
    //                 //     handle_arg(arg, true)?;
    //                 // }
    //                 // if let Some(arg) = nu_sig.rest_positional.clone() {
    //                 //     sig.star_args = true;
    //                 //     handle_arg(arg, false)?;
    //                 // }
    //             }
    //             _ => {} // }
    //                     // dbg!(p.to_string());
    //                     //     for w in p.children(&mut c.walk()) {
    //                     //         if w.kind() == "method_declaration" {
    //                     //             for child in w.children(&mut w.walk()) {
    //                     //                 if child
    //                     //                     .utf8_text(code.as_bytes())
    //                     //                     .map(|name| name == "Main")
    //                     //                     .unwrap_or(false)
    //                     //                 {
    //                     //                     return Some((w, None));
    //                     //                 }
    //                     //             }
    //                     //         }
    //                     //     }
    //                     // }
    //         }
    //     }
    // }
    // // return None;
    // // let no_main_func = Some(main_sig.is_none());
    // let mut is_async = false;
    // let mut is_public = false;
    // let mut returns_void = false;
    // let mut class_name = None;
    Ok(sig)
}
// fn find_typ<'a>(typ_node: Node<'a>, code: &str, top_level: bool) -> anyhow::Result<Typ> {
//     match dbg!(typ_node.kind()) {
//         "flat_type" => {
//             match dbg!(typ_node.utf8_text(code.as_bytes())) {
//                 Ok("float") if !top_level => bail!("`float` is only supported on top level, use `number` instead."),
//                 Ok("int") if !top_level => bail!("`int` is only supported on top level, use `number` instead."),
//                 Ok("datetime") if !top_level => bail!("`datetime` is only supported on top level."),
//                 Ok("binary") if !top_level => bail!("`binary` is only supported on top level."),
//                 Ok("any") | Ok("nothing") => Ok(Typ::Unknown),
//                 Ok("int") => Ok(Typ::Int),
//                 Ok("number") => Ok(Typ::Float),
//                 Ok("float") => Ok(Typ::Float),
//                 Ok("string") => Ok(Typ::Str(None)),
//                 Ok("datetime") => Ok(Typ::Datetime),
//                 Ok("binary") => Ok(Typ::Bytes),
//                 Ok("bool") => Ok(Typ::Bool),
//                 Ok("record") => Ok(Typ::Object(vec![])),
//                 Ok("list") => Ok(Typ::List(Box::new(Typ::Unknown))),
//                 Ok("table") => Ok(Typ::List(Box::new(Typ::Object(vec![])))),
//                 Ok(s) => Err(anyhow!("Unknown type `{s}`")),
//                 Err(e) => Err(anyhow!("Error getting type name: {}", e)),
//             }
//         }
//         "list_type" => {
//             let new_typ_node = typ_node
//                 .child_by_field_name("type")
//                 .ok_or(anyhow!("Failed to find inner type of array type"))?;
//             Ok(Typ::List(Box::new(find_typ(new_typ_node, code, false)?)))
//         }
//         "collection_type" =>
// {
//             let mut curs = typ_node.walk();
//             let mut curs2 = typ_node.walk();
//             let mut fields = vec![];
//             dbg!(typ_node.to_string());
//             // let types = typ_node
//             //     .children_by_field_name("type", &mut curs);

//             let keys = typ_node
//                 .children_by_field_name("key", &mut curs2);

//             for key in keys{
//  // dbg!(key.utf8_text(code.as_bytes()));
//  // dbg!(key.to_string());
//  // dbg!(key.to_string());
//  //
//  //
//              let mut field = ObjectProperty{
//                 key: key.utf8_text(code.as_bytes())?.to_owned(),
//                 typ: Box::new(Typ::Unknown),
//             };

//              if let Some(sibling) = key.next_sibling().and_then(|s| s.next_sibling()) {
//                  if sibling.kind() != "identifier" {
//                     field.typ = Box::new(find_typ(sibling, code, false)?)
//                  }
//              }
//              fields.push(field);
//             }

//             match dbg!(typ_node.utf8_text(code.as_bytes())) {
//                 Ok(s) if s.starts_with("record") => Ok(Typ::Object(fields)),
//                 Ok(s) if s.starts_with("table") => Ok(Typ::List(Box::new(Typ::Object(fields)))),
//                 Ok(s) => Err(anyhow!("Unknown type `{s}`")),
//                 Err(e) => Err(anyhow!("Error getting type name: {}", e)),
//                 }
//             }
//         //     let mut fields = vec![];
//         //     // for ch in typ_node.children(&mut typ_node.walk()) {
//         //     //     dbg!(ch.to_string());
//         //     // }
//         //     // for (key, shape) in vec.into_iter() {
//         //     //     fields.push(
//         //     //         ObjectProperty {
//         //     //             key,
//         //     //             typ: Box::new(glue_types(var_name.clone(), shape, false)?),
//         //     //         }
//         //     //     );
//         //     // }
//         //     fields

//         // Ok(Typ::Object({
//         // }))),
//         "generic_name" => Ok(Typ::Unknown),
//         "pointer_type" => Ok(Typ::Int),
//         // "nullable_type" => {
//         //     let new_typ_node = typ_node
//         //         .child_by_field_name("type")
//         //         .ok_or(anyhow!("Failed to find inner type of nullable_type"))?;
//         //     Ok(find_typ(new_typ_node, code)?)
//         // }
//         wc => Err(anyhow!(
//             "Unexpected C# type node kind: {} for '{}'. This type is not handeled by Windmill, please open an issue if this seems to be an error",
//             wc,
//             typ_node.utf8_text(code.as_bytes())?
//         )),
//     }
// }
// // Function to find the Main method's signature
// // fn find_main_signature<'a>(root_node: Node<'a>, code: &str) -> Option<(Node<'a>, Option<String>)> {}
// fn parse_nu_typ<'a>(
//     param_node: Node<'a>,
//     code: &str,
// ) -> anyhow::Result<(Option<String>, Typ, String)> {
//     let name = dbg!(param_node
//         .child_by_field_name("param_name")
//         .or(param_node.child_by_field_name("param_optional"))
//         .and_then(|n| n
//             .utf8_text(code.as_bytes())
//             .ok()
//             .map(|n| n.replace("?", "")))
//         .unwrap_or("".to_owned()));

//     let mut otyp_node = None;
//     for n in param_node.children(&mut param_node.walk()) {
//         if n.kind() == "param_type" {
//             dbg!(n.to_string());
//             otyp_node = dbg!(n.child_by_field_name("type"));
//         }
//     }
//     let otyp = dbg!(otyp_node
//         .and_then(|n| n.utf8_text(code.as_bytes()).ok())
//         .map(|s| s.to_string()));

//     let typ = if let Some(otyp_n) = otyp_node {
//         find_typ(otyp_n, code, true)?
//     } else {
//         Typ::Unknown
//     };

//     Ok((otyp, typ, name.to_string()))
//     // let name = dbg!(param_node
//     //     .child_by_field_name("param_name")
//     //     .and_then(|n| n.utf8_text(code.as_bytes()).ok())
//     //     .unwrap_or(""));

//     // let mut otyp_node = None;
//     // for n in param_node.children(&mut param_node.walk()) {
//     //     if n.kind() == "param_type" {
//     //         dbg!(n.to_string());
//     //         otyp_node = dbg!(n.child_by_field_name("type"));
//     //     }
//     // }
//     // let otyp = dbg!(otyp_node
//     //     .and_then(|n| n.utf8_text(code.as_bytes()).ok())
//     //     .map(|s| s.to_string()));

//     // let typ = if let Some(otyp_n) = otyp_node {
//     //     find_typ(otyp_n, code)?
//     // } else {
//     //     Typ::Unknown
//     // };

//     // Ok((otyp, typ, name.to_string()))
// }
// //     // let engine_state = nu_cmd_lang::create_default_context();
// //     let engine_state = EngineState::new();
// //     let mut set = StateWorkingSet::new(&engine_state);
// //     let block = { &nu_parser::parse(&mut set, None, code.as_bytes(), false) };

// //     let mut sig = MainArgSignature {
// //         no_main_func: Some(true),
// //         ..Default::default() //
// //     };

// //     for pipeline in &block.pipelines {
// //         for el in &pipeline.elements {
// //             if let Expr::Call(ref call) = el.expr.expr {
// //                 let mut iter = call.positional_iter();
// //                 match (iter.next(), iter.next()) {
// //                     (
// //                         Some(Expression { expr: Expr::String(fn_name), .. }),
// //                         Some(Expression { expr: Expr::Signature(nu_sig), .. }),
// //                     ) if fn_name == "main" => {
// //                         sig.no_main_func = Some(false);
// //                         let mut handle_arg =
// //                             |PositionalArg { name, desc: _, shape, var_id: _, default_value },
// //                              has_default|
// //                              -> anyhow::Result<()> {
// //                                 let or = if has_default { Some(json!(null)) } else { None };
// //                                 sig.args.push(Arg {
// //                                     name: name.clone(),
// //                                     typ: glue_types(name, shape, true)?,
// //                                     otyp: None,
// //                                     default: default_value
// //                                         .and_then(|val| parse_default_value(val).ok())
// //                                         .or(or),
// //                                     has_default,
// //                                     oidx: None,
// //                                 });
// //                                 Ok(())
// //                             };
// //                         for arg in nu_sig.required_positional.clone() {
// //                             handle_arg(arg, false)?;
// //                         }
// //                         for arg in nu_sig.optional_positional.clone() {
// //                             handle_arg(arg, true)?;
// //                         }
// //                         if let Some(arg) = nu_sig.rest_positional.clone() {
// //                             sig.star_args = true;
// //                             handle_arg(arg, false)?;
// //                         }
// //                     }
// //                     _ => {}
// //                 }
// //             }
// //         }
// //     }
// //     Ok(sig)
// // }

// // fn parse_default_value(val: nu_protocol::Value) -> anyhow::Result<serde_json::Value> {
// //     use nu_protocol::Value::*;
// //     use serde_json::to_value;
// //     match val {
// //         Bool { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         Int { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         // Number { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         Float { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         String { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         Date { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         Record { val, .. } => Ok(serde_json::Value::Object(Map::from_iter({
// //             let mut fields = vec![];
// //             for (name, val) in <nu_protocol::Record as Clone>::clone(&val) {
// //                 fields.push((name, parse_default_value(val)?));
// //             }
// //             fields.into_iter()
// //         }))),
// //         List { vals, .. } => {
// //             let mut json_values = vec![];
// //             for val in vals.into_iter() {
// //                 json_values.push(parse_default_value(val)?);
// //             }
// //             Ok(serde_json::Value::Array(json_values))
// //         }
// //         Nothing { .. } => Ok(json!("null")),
// //         Binary { val, .. } => to_value(val).map_err(anyhow::Error::from),
// //         wc => Err(anyhow::anyhow!(
// //             "Unexpected Nu type node kind: {:?}. This type is not handled by Windmill, please open an issue if this seems to be an error",
// //             wc,
// //         )),
// //     }
// // }

// // use windmill_parser::Typ;
// // fn glue_types(var_name: String, shape: SyntaxShape, is_top_level: bool) -> anyhow::Result<Typ> {
// //     use nu_protocol::SyntaxShape::*;
// //     Ok(match shape {
// //         Any | Nothing => Typ::Unknown,
// //         Number => Typ::Float,
// //         Boolean => Typ::Bool,
// //         String => Typ::Str(None),

// //         Record(vec) => Typ::Object({
// //             let mut fields = vec![];
// //             for (key, shape) in vec.into_iter() {
// //                 fields.push(
// //                     ObjectProperty {
// //                         key,
// //                         typ: Box::new(glue_types(var_name.clone(), shape, false)?),
// //                     }
// //                 );
// //             }
// //             fields
// //         }),
// //         Table(vec) => Typ::List(Box::new(
// //             Typ::Object({
// //                 let mut fields = vec![];
// //                 for (key, shape) in vec.into_iter() {
// //                     fields.push(
// //                         ObjectProperty {
// //                             key,
// //                             typ: Box::new(glue_types(var_name.clone(), shape, false)?),
// //                         }
// //                     );
// //                 }
//                 fields
//             })
//         )),

//         List(syntax_shape) => Typ::List(Box::new(glue_types(var_name, *syntax_shape, false)?)),
//         Float if !is_top_level => bail!("arg: {var_name}\n `float` is only supported on top level, use `number` instead."),
//         Int if !is_top_level => bail!("arg: {var_name}\n `int` is only supported on top level, use `number` instead."),
//         t if !is_top_level => bail!("arg: {var_name}\n `{t}` is only supported on top level."),

//         Binary => Typ::Bytes,
//         DateTime => Typ::Datetime,
//         Float => Typ::Float,
//         Int => Typ::Int,
//         t => bail!("arg: {var_name}\n `{t}` is not handled by Windmill, please open an issue if this seems to be an error"),
//     })
// }

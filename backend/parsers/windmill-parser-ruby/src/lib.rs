#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::anyhow;
use anyhow::bail;
use regex::Regex;
use serde_json::Value;
use tree_sitter::Node;
use tree_sitter::Range;
use windmill_parser::json_to_typ;
use windmill_parser::Arg;
use windmill_parser::MainArgSignature;

pub fn parse_ruby_sig_meta(code: &str) -> anyhow::Result<MainArgSignature> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_ruby::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting Ruby as language: {e}"))?;

    // Parse code
    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Failed to parse code"))?;
    let root_node = tree.root_node();

    root_node.clone().to_string();
    // Traverse the AST to find the Main method signature
    let args = find_main_signature(root_node, code)?;
    let no_main_func = Some(args.is_none());

    let main_sig = MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args: args.unwrap_or_default(),
        has_preprocessor: None,
        no_main_func,
    };

    Ok(main_sig)
}

pub fn parse_ruby_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    Ok(parse_ruby_sig_meta(code)?)
}

pub fn parse_ruby_requirements(code: &str) -> anyhow::Result<String> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_ruby::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting Ruby as language: {e}"))?;

    // Parse code
    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Failed to parse code"))?;
    let root_node = tree.root_node();

    root_node.clone().to_string();
    // TODO: Parses requirements. For now disabled in favour of inline dependencies

    let mut cursor = root_node.walk();
    'top_level: for x in root_node.children(&mut cursor) {
        dbg!(&x);
        if x.kind() == "call" {
            for (i, n) in x.children(&mut x.walk()).enumerate() {
                if i == 0
                    && n.kind() == "identifier"
                    && !dbg!(n.utf8_text(code.as_bytes()))
                        .map(|ident| ident == "gemfile")
                        .unwrap_or_default()
                // TODO: Infer inputs from imports automatically
                // && !dbg!(n.utf8_text(code.as_bytes()))
                //     .map(|ident| ident == "require")
                //     .unwrap_or_default()
                // {
                //     continue 'top_level;
                // } else if dbg!(n.kind()) == "argument_list" {
                //     let req = n.utf8_text(code.as_bytes())?.to_owned();

                //     if req.starts_with(['u', 'f']) {
                //         reqs.push(req);
                //     } else if let Some(root) = req.split('/').next() {
                //         reqs.push(root.to_owned());
                //     }
                // }
                {
                    continue 'top_level;
                } else if dbg!(n.kind()) == "do_block" {
                    let req = n.utf8_text(code.as_bytes())?.to_owned();

                    lazy_static::lazy_static! {
                        static ref WINDMILL_RE: Regex = Regex::new(r"^\s*require\s*'windmill/inline'").unwrap();
                    }

                    if WINDMILL_RE.find(&code).is_none() {
                        return Err(anyhow!(
                            "`require 'windmill/inline'` is not detected - please add `require 'windmill/inline'` in order to use inline gemfile.
                Your Gemfile syntax will be compatible with bundler/inline."
                        )
                        .into());
                    }

                    return req
                        .get(2..(req.len() - 3))
                        .map(str::to_owned)
                        .ok_or(anyhow!("Invalid gemfile do block"));

                    // TODO: Relative imports
                    // if req.starts_with(['u', 'f']) {
                    //     reqs.push(req);
                    // } else if let Some(root) = req.split('/').next() {
                    //     reqs.push(root.to_owned());
                    // }
                }
            }
        }
    }
    Ok(String::new())
}
// pub fn parse_ruby_requirements(code: &str) -> anyhow::Result<Vec<String>> {
//     let mut reqs = vec![];
//     let mut parser = tree_sitter::Parser::new();
//     let language = tree_sitter_ruby::LANGUAGE;
//     parser
//         .set_language(&language.into())
//         .map_err(|e| anyhow!("Error setting Ruby as language: {e}"))?;

//     // Parse code
//     let tree = parser
//         .parse(code, None)
//         .ok_or(anyhow!("Failed to parse code"))?;
//     let root_node = tree.root_node();

//     root_node.clone().to_string();
//     // TODO: Parses requirements. For now disabled in favour of inline dependencies

//     let mut cursor = root_node.walk();
//     'top_level: for x in root_node.children(&mut cursor) {
//         dbg!(&x);
//         if x.kind() == "call" {
//             for (i, n) in x.children(&mut x.walk()).enumerate() {
//                 if i == 0
//                     && n.kind() == "identifier"
//                     && !dbg!(n.utf8_text(code.as_bytes()))
//                         .map(|ident| ident == "gemfile")
//                         .unwrap_or_default()
//                 // TODO: Infer inputs from imports automatically
//                 // && !dbg!(n.utf8_text(code.as_bytes()))
//                 //     .map(|ident| ident == "require")
//                 //     .unwrap_or_default()
//                 // {
//                 //     continue 'top_level;
//                 // } else if dbg!(n.kind()) == "argument_list" {
//                 //     let req = n.utf8_text(code.as_bytes())?.to_owned();

//                 //     if req.starts_with(['u', 'f']) {
//                 //         reqs.push(req);
//                 //     } else if let Some(root) = req.split('/').next() {
//                 //         reqs.push(root.to_owned());
//                 //     }
//                 // }
//                 {
//                     continue 'top_level;
//                 } else if dbg!(n.kind()) == "do_block" {
//                     let req = n.utf8_text(code.as_bytes())?.to_owned();

//                     dbg!(req.get(1..(req.len() - 2)));

//                     // TODO: Relative imports
//                     // if req.starts_with(['u', 'f']) {
//                     //     reqs.push(req);
//                     // } else if let Some(root) = req.split('/').next() {
//                     //     reqs.push(root.to_owned());
//                     // }
//                 }
//             }
//         }
//     }
//     Ok(reqs)
// }

// Function to find the Main method's signature
fn find_main_signature<'a>(root_node: Node<'a>, code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut cursor = root_node.walk();
    'top_level: for x in root_node.children(&mut cursor) {
        if x.kind() == "method" {
            let mut args = vec![];
            for (i, m) in x.children(&mut x.walk()).skip(1).enumerate() {
                if i == 0
                    && m.kind() == "identifier"
                    && !m
                        .utf8_text(code.as_bytes())
                        .map(|ident| ident == "main")
                        .unwrap_or_default()
                {
                    continue 'top_level;
                } else if m.kind() == "method_parameters" {
                    for a in m.children(&mut m.walk()) {
                        if a.kind() == "identifier" {
                            let name = a.utf8_text(code.as_bytes()).unwrap();
                            args.push(Arg { name: name.into(), ..Default::default() });
                        }
                        if a.kind() == "optional_parameter" {
                            let mut walk = a.walk();
                            let mut it = a.children(&mut walk).into_iter();
                            match (it.next().and_then(|n| n.utf8_text(code.as_bytes()).ok()), {
                                // Skip `=`
                                it.next();
                                it.next().map(|n| n.range())
                            }) {
                                (Some(ident), Some(Range { start_byte, end_byte, .. })) => {
                                    let unparsed =
                                        &code[start_byte..end_byte].replace("nil", "null");
                                    let default: Value = serde_json::from_str(unparsed).map_err(
                                        |e|
                                        anyhow!("Cannot convert default value to json\n\tvalue: {unparsed}\n\terror: {e}{}",
                                            if e.to_string().contains("key must be a string") {
                                                "\n\nNOTE: If you are trying to declare default hash, use following syntax:\n { \"<key>\": <value> }"
                                            } else {
                                                ""
                                            }
                                        ))?;
                                    args.push(Arg {
                                        name: ident.to_owned(),
                                        typ: json_to_typ(&default, true),
                                        default: Some(default),
                                        has_default: true,
                                        ..Default::default()
                                    });
                                }
                                _ => {
                                    let Range { start_byte, end_byte, .. } = a.range();
                                    bail!(
                                        "Cannot parse optional parameter: {}",
                                        &code.get(start_byte..end_byte).unwrap_or("CANNOT DISPLAY")
                                    )
                                }
                            }
                        }
                        let kind = a.kind();
                        if matches!(
                            kind,
                            "keyword_parameter" | "splat_parameter" | "hash_splat_parameter"
                        ) {
                            let Range { start_byte, end_byte, .. } = a.range();
                            bail!(
                                " - {}\n{}s are not supported",
                                &code.get(start_byte..end_byte).unwrap_or("CANNOT DISPLAY"),
                                kind.replace("_", " ")
                            );
                        }
                    }
                }
            }
            return Ok(Some(args));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use windmill_parser::{ObjectProperty, Typ};

    use super::parse_ruby_sig_meta as parse;
    use super::*;
    #[test]
    fn test_parse_ruby_no_main() {
        let code = r#"
def not_main end
def private_fn end
"#;
        let sig = parse(code).unwrap();

        assert_eq!(
            sig,
            MainArgSignature { no_main_func: Some(true), ..Default::default() }
        );
    }
    #[test]
    fn test_parse_ruby_no_args() {
        let code = r#"
def main
end
"#;
        let sig = parse(code).unwrap();

        assert_eq!(
            sig,
            MainArgSignature { no_main_func: Some(false), ..Default::default() }
        );
    }
    #[test]
    fn test_parse_ruby_positional_args() {
        let sig = {
            let code = r#"def main(a, b, c) end"#;
            parse(code).unwrap()
        };
        let sig2 = {
            let code = r#"
            def main a, b, c
            end"#;
            parse(code).unwrap()
        };

        assert_eq!(
            sig,
            MainArgSignature {
                args: vec![
                    Arg { name: "a".into(), ..Default::default() },
                    Arg { name: "b".into(), ..Default::default() },
                    Arg { name: "c".into(), ..Default::default() }
                ],
                no_main_func: Some(false),
                ..Default::default()
            }
        );
        assert_eq!(sig2, sig);
    }
    #[test]
    fn test_parse_ruby_lists() {
        let sig = {
            let code = r#"def main(a = [ 1, 2, 3 ], b = [ 1, false, nil, "test"]) end"#;
            parse(code).unwrap()
        };
        assert_eq!(
            sig,
            MainArgSignature {
                args: vec![
                    Arg {
                        name: "a".into(),
                        has_default: true,
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2, 3])),
                        ..Default::default()
                    },
                    Arg {
                        name: "b".into(),
                        has_default: true,
                        typ: Typ::List(Box::new(Typ::Unknown)),
                        default: Some(json!([1, false, null, "test"])),
                        ..Default::default()
                    },
                ],
                no_main_func: Some(false),
                ..Default::default()
            }
        );
    }
    #[test]
    fn test_parse_ruby_hashes() {
        let sig = {
            let code = r#"def main(a = { "1": 4, "2": [ 1, 2, 3] }) end"#;
            parse(code).unwrap()
        };
        assert_eq!(
            sig,
            MainArgSignature {
                args: vec![Arg {
                    name: "a".into(),
                    has_default: true,
                    typ: Typ::Object(vec![
                        ObjectProperty { key: "1".into(), typ: Box::new(Typ::Int) },
                        ObjectProperty {
                            key: "2".into(),
                            typ: Box::new(Typ::List(Box::new(Typ::Int)))
                        }
                    ],),
                    default: Some(json!({"1": 4, "2": [ 1, 2, 3 ]})),
                    ..Default::default()
                },],
                no_main_func: Some(false),
                ..Default::default()
            }
        );
    }
    #[test]
    fn test_parse_ruby_default_args() {
        let sig = {
            let code =
                r#"def main(a = 10, b = "hey", c = false, d = [ 1, 2, 3 ], e = { "a": 43 }) end"#;
            parse(code).unwrap()
        };

        assert_eq!(
            sig,
            MainArgSignature {
                args: vec![
                    Arg {
                        name: "a".into(),
                        has_default: true,
                        typ: Typ::Int,
                        default: Some(json!(10)),
                        ..Default::default()
                    },
                    Arg {
                        name: "b".into(),
                        has_default: true,
                        typ: Typ::Str(None),
                        default: Some(json!("hey")),
                        ..Default::default()
                    },
                    Arg {
                        name: "c".into(),
                        has_default: true,
                        typ: Typ::Bool,
                        default: Some(json!(false)),
                        ..Default::default()
                    },
                    Arg {
                        name: "d".into(),
                        has_default: true,
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2, 3])),
                        ..Default::default()
                    },
                    Arg {
                        name: "e".into(),
                        has_default: true,
                        typ: Typ::Object(vec![ObjectProperty {
                            key: "a".into(),
                            typ: Box::new(Typ::Int)
                        }]),
                        default: Some(json!({"a": 43})),
                        ..Default::default()
                    },
                ],
                no_main_func: Some(false),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_ruby_unsupported() {
        assert_eq!(
            &parse("def main( a: ) end").unwrap_err().to_string(),
            " - a:\nkeyword parameters are not supported"
        );

        assert_eq!(
            &parse("def main( *a ) end").unwrap_err().to_string(),
            " - *a\nsplat parameters are not supported"
        );

        assert_eq!(
            &parse("def main( **a ) end").unwrap_err().to_string(),
            " - **a\nhash splat parameters are not supported"
        );

        assert!(&parse("def main( a = Time.now ) end").is_err());
        assert!(&parse("def main( a = :symbol ) end").is_err());
        assert!(&parse("def main( a = { b: 2 } ) end").is_err());
        assert!(&parse("def main( a = { b => 2 } ) end").is_err());
        assert!(&parse(r#"def main( a = { "b" => 2 } ) end"#).is_err());
    }

    //     #[test]
    //     fn test_parse_ruby_requirements() {
    //         assert_eq!(
    //             parse_ruby_requirements(
    //                 "
    // require 'dep1'
    // require 'dep2'
    // require 'dep2/submod'
    // require 'u/username/script'
    // require 'f/folder/script'
    //                 "
    //             )
    //             .unwrap(),
    //             vec![
    //                 "dep1".to_owned(),
    //                 "dep2".into(),
    //                 "dep2".into(),
    //                 "u/username/script".into(),
    //                 "f/folder/script".into(),
    //             ]
    //         );
    //     }
}

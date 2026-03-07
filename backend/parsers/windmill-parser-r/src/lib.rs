#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::anyhow;
use serde_json::Value;
use tree_sitter::Node;
use tree_sitter::Range;
use windmill_parser::json_to_typ;
use windmill_parser::Arg;
use windmill_parser::MainArgSignature;

pub fn parse_r_sig_meta(code: &str) -> anyhow::Result<MainArgSignature> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_r::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting R as language: {e}"))?;

    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Failed to parse code"))?;
    let root_node = tree.root_node();

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

pub fn parse_r_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    Ok(parse_r_sig_meta(code)?)
}

/// Find the main function signature in R code.
/// R function definitions look like: `main <- function(x, y = 10) { ... }`
/// In the tree-sitter-r AST, this is a `binary_operator` node with:
///   - child 0: identifier "main"
///   - child 1: "<-" or "="
///   - child 2: function_definition node
fn find_main_signature<'a>(root_node: Node<'a>, code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut cursor = root_node.walk();
    for x in root_node.children(&mut cursor) {
        if x.kind() == "binary_operator" {
            let child_count = x.child_count();
            if child_count < 3 {
                continue;
            }

            // First child should be identifier "main"
            let ident_node = x.child(0).unwrap();
            if ident_node.kind() != "identifier" {
                continue;
            }
            let ident = ident_node.utf8_text(code.as_bytes()).unwrap_or("");
            if ident != "main" {
                continue;
            }

            // Second child should be "<-" or "="
            let op_node = x.child(1).unwrap();
            let op = op_node.utf8_text(code.as_bytes()).unwrap_or("");
            if op != "<-" && op != "=" {
                continue;
            }

            // Third child should be the function_definition
            let func_node = x.child(2).unwrap();
            if func_node.kind() != "function_definition" {
                continue;
            }

            return Ok(Some(parse_function_params(func_node, code)?));
        }
    }
    Ok(None)
}

/// Parse parameters from a function_definition node.
/// function_definition has children: "function", parameters, body
/// Each parameter node has:
///   - 1 child (identifier) for positional args
///   - 3 children (identifier, "=", value) for default args
fn parse_function_params(func_node: Node, code: &str) -> anyhow::Result<Vec<Arg>> {
    let mut args = vec![];
    let mut func_cursor = func_node.walk();

    for child in func_node.children(&mut func_cursor) {
        if child.kind() == "parameters" {
            let mut param_cursor = child.walk();
            for param in child.children(&mut param_cursor) {
                if param.kind() != "parameter" {
                    continue;
                }

                let param_child_count = param.child_count();
                if param_child_count == 1 {
                    // Simple parameter: just identifier
                    let ident_node = param.child(0).unwrap();
                    let name = ident_node.utf8_text(code.as_bytes())?;
                    args.push(Arg {
                        name: name.to_owned(),
                        ..Default::default()
                    });
                } else if param_child_count >= 3 {
                    // Default parameter: identifier = value
                    let ident_node = param.child(0).unwrap();
                    let value_node = param.child(2).unwrap();
                    let name = ident_node.utf8_text(code.as_bytes())?;

                    let Range { start_byte, end_byte, .. } = value_node.range();
                    let raw = &code[start_byte..end_byte];
                    // Convert R literals to JSON
                    let unparsed = raw
                        .replace("NULL", "null")
                        .replace("TRUE", "true")
                        .replace("FALSE", "false");
                    match serde_json::from_str::<Value>(&unparsed) {
                        Ok(default) => {
                            args.push(Arg {
                                name: name.to_owned(),
                                typ: json_to_typ(&default, true),
                                default: Some(default),
                                has_default: true,
                                ..Default::default()
                            });
                        }
                        Err(_) => {
                            args.push(Arg {
                                name: name.to_owned(),
                                has_default: true,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(args)
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use windmill_parser::Typ;

    use super::parse_r_sig_meta as parse;

    #[test]
    fn test_parse_r_no_main() {
        let code = r#"
not_main <- function() {}
helper <- function(x) { x + 1 }
"#;
        let sig = parse(code).unwrap();
        assert_eq!(
            sig,
            windmill_parser::MainArgSignature {
                no_main_func: Some(true),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_r_no_args() {
        let code = r#"
main <- function() {
    return(42)
}
"#;
        let sig = parse(code).unwrap();
        assert_eq!(
            sig,
            windmill_parser::MainArgSignature {
                no_main_func: Some(false),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_r_positional_args() {
        let code = r#"main <- function(a, b, c) { a + b + c }"#;
        let sig = parse(code).unwrap();
        assert_eq!(
            sig,
            windmill_parser::MainArgSignature {
                args: vec![
                    windmill_parser::Arg { name: "a".into(), ..Default::default() },
                    windmill_parser::Arg { name: "b".into(), ..Default::default() },
                    windmill_parser::Arg { name: "c".into(), ..Default::default() },
                ],
                no_main_func: Some(false),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_r_default_args() {
        let code = r#"main <- function(a = 10, b = "hey", c = FALSE) { }"#;
        let sig = parse(code).unwrap();
        assert_eq!(sig.args.len(), 3);
        assert_eq!(sig.args[0].name, "a");
        assert_eq!(sig.args[0].default, Some(json!(10)));
        assert_eq!(sig.args[0].typ, Typ::Int);
        assert_eq!(sig.args[1].name, "b");
        assert_eq!(sig.args[1].default, Some(json!("hey")));
        assert_eq!(sig.args[1].typ, Typ::Str(None));
        assert_eq!(sig.args[2].name, "c");
        assert_eq!(sig.args[2].default, Some(json!(false)));
        assert_eq!(sig.args[2].typ, Typ::Bool);
    }

    #[test]
    fn test_parse_r_equals_assignment() {
        let code = r#"main = function(x, y = 5) { x + y }"#;
        let sig = parse(code).unwrap();
        assert_eq!(sig.args.len(), 2);
        assert_eq!(sig.args[0].name, "x");
        assert_eq!(sig.args[1].name, "y");
        assert_eq!(sig.args[1].default, Some(json!(5)));
    }

    #[test]
    fn test_parse_r_null_default() {
        let code = r#"main <- function(x = NULL) { x }"#;
        let sig = parse(code).unwrap();
        assert_eq!(sig.args.len(), 1);
        assert_eq!(sig.args[0].name, "x");
        assert_eq!(sig.args[0].default, Some(json!(null)));
    }

}

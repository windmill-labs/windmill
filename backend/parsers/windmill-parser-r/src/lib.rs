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

/// Extract package names from `library(...)` and `require(...)` calls in R code.
/// Returns a newline-separated list of package names.
pub fn parse_r_requirements(code: &str) -> anyhow::Result<String> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_r::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting R as language: {e}"))?;

    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Failed to parse code"))?;
    let root_node = tree.root_node();

    let mut packages = vec![];
    find_library_calls(root_node, code, &mut packages);

    // Deduplicate and exclude base packages
    packages.sort();
    packages.dedup();
    packages.retain(|p| !is_base_package(p));

    Ok(packages.join("\n"))
}

fn find_library_calls(node: Node, code: &str, packages: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call" {
            // call node: child 0 is the function name, child 1 is arguments
            if let (Some(func_node), Some(args_node)) = (child.child(0), child.child(1)) {
                let func_name = func_node.utf8_text(code.as_bytes()).unwrap_or("");
                if func_name == "library" || func_name == "require" {
                    // AST: arguments → ( + argument → identifier/string + )
                    if args_node.kind() == "arguments" {
                        let mut args_cursor = args_node.walk();
                        for arg in args_node.children(&mut args_cursor) {
                            if arg.kind() == "argument" {
                                // The argument node wraps the actual value
                                if let Some(value_node) = arg.child(0) {
                                    let pkg = value_node
                                        .utf8_text(code.as_bytes())
                                        .unwrap_or("")
                                        .trim_matches('"')
                                        .trim_matches('\'');
                                    if !pkg.is_empty() {
                                        packages.push(pkg.to_string());
                                    }
                                }
                                break; // only first arg
                            }
                        }
                    }
                }
            }
        }
        // Recurse into children to find nested library() calls
        find_library_calls(child, code, packages);
    }
}

fn is_base_package(pkg: &str) -> bool {
    matches!(
        pkg,
        "base"
            | "compiler"
            | "datasets"
            | "grDevices"
            | "graphics"
            | "grid"
            | "methods"
            | "parallel"
            | "splines"
            | "stats"
            | "stats4"
            | "tcltk"
            | "tools"
            | "utils"
    )
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
                    args.push(Arg { name: name.to_owned(), ..Default::default() });
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
            windmill_parser::MainArgSignature { no_main_func: Some(true), ..Default::default() }
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
            windmill_parser::MainArgSignature { no_main_func: Some(false), ..Default::default() }
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

    #[test]
    fn test_parse_r_requirements() {
        use super::parse_r_requirements;

        let code = r#"
library(dplyr)
library(ggplot2)
require(tidyr)
library(stats)

main <- function(x) {
    library(stringr)
    x
}
"#;
        let reqs = parse_r_requirements(code).unwrap();
        let pkgs: Vec<&str> = reqs.lines().collect();
        assert!(pkgs.contains(&"dplyr"));
        assert!(pkgs.contains(&"ggplot2"));
        assert!(pkgs.contains(&"tidyr"));
        assert!(pkgs.contains(&"stringr"));
        assert!(!pkgs.contains(&"stats")); // base package excluded
    }

    #[test]
    fn test_parse_r_requirements_string_args() {
        use super::parse_r_requirements;

        let code = r#"
library("data.table")
require("jsonlite")

main <- function() { }
"#;
        let reqs = parse_r_requirements(code).unwrap();
        let pkgs: Vec<&str> = reqs.lines().collect();
        assert!(pkgs.contains(&"data.table"));
        assert!(pkgs.contains(&"jsonlite"));
    }

    #[test]
    fn test_parse_r_requirements_no_deps() {
        use super::parse_r_requirements;

        let code = r#"main <- function(x) { x + 1 }"#;
        let reqs = parse_r_requirements(code).unwrap();
        assert!(reqs.is_empty());
    }
}

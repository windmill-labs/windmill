use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
    pub parent: Option<String>,
}

pub enum Lang {
    Rust,
    Typescript,
}

impl Lang {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "ts" | "tsx" | "js" | "jsx" => Some(Self::Typescript),
            "svelte" => Some(Self::Typescript), // we extract <script> block
            _ => None,
        }
    }
}

pub fn parse_file(path: &Path) -> Result<Vec<Symbol>> {
    let lang = Lang::from_path(path).context("unsupported file type")?;
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;

    let code = match lang {
        Lang::Typescript if path.extension().map(|e| e == "svelte").unwrap_or(false) => {
            extract_svelte_script(&source)
        }
        _ => source.clone(),
    };

    let mut parser = Parser::new();
    match lang {
        Lang::Rust => {
            parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
        }
        Lang::Typescript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        }
    }

    let tree = parser
        .parse(&code, None)
        .context("tree-sitter parse failed")?;
    let root = tree.root_node();

    let mut symbols = Vec::new();
    match lang {
        Lang::Rust => extract_rust_symbols(root, &code, &mut symbols, None),
        Lang::Typescript => extract_ts_symbols(root, &code, &mut symbols, None),
    }
    Ok(symbols)
}

fn extract_svelte_script(source: &str) -> String {
    // Find <script ...> ... </script> blocks and extract content
    let mut result = String::new();
    let mut remaining = source;
    while let Some(start) = remaining.find("<script") {
        if let Some(tag_end) = remaining[start..].find('>') {
            let content_start = start + tag_end + 1;
            if let Some(end) = remaining[content_start..].find("</script>") {
                result.push_str(&remaining[content_start..content_start + end]);
                result.push('\n');
                remaining = &remaining[content_start + end + 9..];
                continue;
            }
        }
        break;
    }
    result
}

fn extract_rust_symbols(node: Node, source: &str, symbols: &mut Vec<Symbol>, parent: Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(sym) = rust_function(child, source, parent) {
                    symbols.push(sym);
                }
            }
            "struct_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let sig = signature_up_to_body(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: "struct".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(sig),
                        parent: parent.map(String::from),
                    });
                }
            }
            "enum_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let sig = signature_up_to_body(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: "enum".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(sig),
                        parent: parent.map(String::from),
                    });
                }
            }
            "trait_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let sig = signature_up_to_body(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: "trait".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(sig),
                        parent: parent.map(String::from),
                    });
                    // Recurse into trait body for methods
                    if let Some(body) = child.child_by_field_name("body") {
                        extract_rust_symbols(body, source, symbols, Some(&name));
                    }
                }
            }
            "impl_item" => {
                let impl_name = rust_impl_name(child, source);
                let sig = signature_up_to_body(child, source);
                let parent_name = impl_name.as_deref().unwrap_or("impl");
                symbols.push(Symbol {
                    name: parent_name.to_string(),
                    kind: "impl".into(),
                    line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    signature: Some(sig),
                    parent: parent.map(String::from),
                });
                // Recurse into impl body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    extract_rust_symbols(body, source, symbols, Some(parent_name));
                }
            }
            "type_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "type_alias".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(node_text(child, source).to_string()),
                        parent: parent.map(String::from),
                    });
                }
            }
            "const_item" | "static_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: child.kind().trim_end_matches("_item").into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(signature_up_to_body(child, source)),
                        parent: parent.map(String::from),
                    });
                }
            }
            "mod_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "mod".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: None,
                        parent: parent.map(String::from),
                    });
                }
            }
            "macro_definition" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "macro".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: None,
                        parent: parent.map(String::from),
                    });
                }
            }
            // Recurse into declaration_list (impl body, trait body)
            "declaration_list" => {
                extract_rust_symbols(child, source, symbols, parent);
            }
            _ => {}
        }
    }
}

fn rust_function(node: Node, source: &str, parent: Option<&str>) -> Option<Symbol> {
    let name = child_by_field(node, "name", source)?;
    let sig = rust_fn_signature(node, source);
    Some(Symbol {
        name,
        kind: "function".into(),
        line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        signature: Some(sig),
        parent: parent.map(String::from),
    })
}

fn rust_fn_signature(node: Node, source: &str) -> String {
    // Capture everything up to the block (the `{`)
    let text = node_text(node, source);
    if let Some(brace) = text.find('{') {
        text[..brace].trim().to_string()
    } else {
        // No body (trait declaration)
        text.lines().next().unwrap_or("").to_string()
    }
}

fn rust_impl_name(node: Node, source: &str) -> Option<String> {
    // impl [Trait for] Type
    let mut cursor = node.walk();
    let mut type_name = None;
    let mut trait_name = None;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" | "scoped_type_identifier" | "generic_type" => {
                if trait_name.is_none() && type_name.is_none() {
                    type_name = Some(node_text(child, source).to_string());
                } else if type_name.is_some() {
                    // This is the type after "for"
                    trait_name = type_name.take();
                    type_name = Some(node_text(child, source).to_string());
                }
            }
            _ => {}
        }
    }
    match (trait_name, type_name) {
        (Some(t), Some(ty)) => Some(format!("{t} for {ty}")),
        (None, Some(ty)) => Some(ty),
        _ => None,
    }
}

fn extract_ts_symbols(node: Node, source: &str, symbols: &mut Vec<Symbol>, parent: Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "function".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(signature_up_to_body(child, source)),
                        parent: parent.map(String::from),
                    });
                }
            }
            "interface_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "interface".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(signature_up_to_body(child, source)),
                        parent: parent.map(String::from),
                    });
                }
            }
            "type_alias_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "type_alias".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(node_text(child, source).to_string()),
                        parent: parent.map(String::from),
                    });
                }
            }
            "enum_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: "enum".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(signature_up_to_body(child, source)),
                        parent: parent.map(String::from),
                    });
                }
            }
            "class_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: "class".into(),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        signature: Some(signature_up_to_body(child, source)),
                        parent: parent.map(String::from),
                    });
                    if let Some(body) = child.child_by_field_name("body") {
                        extract_ts_symbols(body, source, symbols, Some(&name));
                    }
                }
            }
            "export_statement" | "program" => {
                extract_ts_symbols(child, source, symbols, parent);
            }
            "lexical_declaration" => {
                // const foo = ... or let foo = ...
                ts_variable_decl(child, source, symbols, parent);
            }
            _ => {}
        }
    }
}

fn ts_variable_decl(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    parent: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name) = child_by_field(child, "name", source) {
                // Check if the value is an arrow function or function expression
                let kind = if let Some(value) = child.child_by_field_name("value") {
                    match value.kind() {
                        "arrow_function" | "function_expression" | "function" => "function",
                        _ => "const",
                    }
                } else {
                    "const"
                };
                symbols.push(Symbol {
                    name,
                    kind: kind.into(),
                    line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    signature: Some(signature_up_to_body(node, source)),
                    parent: parent.map(String::from),
                });
            }
        }
    }
}

fn child_by_field(node: Node, field: &str, source: &str) -> Option<String> {
    Some(node_text(node.child_by_field_name(field)?, source).to_string())
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn signature_up_to_body(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    // Find first `{` that starts a block body
    if let Some(pos) = text.find('{') {
        let sig = text[..pos].trim();
        // Collapse whitespace
        sig.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        let first_line = text.lines().next().unwrap_or("");
        first_line.trim().to_string()
    }
}

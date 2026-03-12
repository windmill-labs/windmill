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

#[derive(Debug, Clone)]
pub struct IdentRef {
    pub name: String,
    pub line: usize,
    /// Resolved import path if known (e.g. "windmill_common::error::Error")
    pub import_path: Option<String>,
}

/// A `use` import with its scope
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// The short name (e.g. "Error")
    pub name: String,
    /// Full path (e.g. "windmill_common::error::Error")
    pub full_path: String,
    /// Line where the use is declared
    pub line: usize,
    /// End of the scope this use lives in (file end for top-level, block end for scoped)
    pub scope_end: usize,
}

pub struct ParseResult {
    pub symbols: Vec<Symbol>,
    pub refs: Vec<IdentRef>,
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

pub fn parse_file(path: &Path) -> Result<ParseResult> {
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

    let mut imports = Vec::new();
    let file_end = code.lines().count();
    match lang {
        Lang::Rust => collect_rust_imports(root, &code, &mut imports, file_end),
        Lang::Typescript => collect_ts_imports(root, &code, &mut imports, file_end),
    }

    let mut refs = Vec::new();
    collect_ident_refs(root, &code, &mut refs);

    // Resolve import paths for refs
    resolve_refs(&mut refs, &imports);

    Ok(ParseResult { symbols, refs })
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

/// Collect all identifier references in code, skipping comments and strings.
fn collect_ident_refs(node: Node, source: &str, refs: &mut Vec<IdentRef>) {
    match node.kind() {
        // Skip non-code nodes
        "line_comment" | "block_comment" | "string_literal" | "raw_string_literal"
        | "string" | "template_string" | "string_fragment" | "comment"
        | "string_content" | "char_literal" => return,
        _ => {}
    }

    if is_identifier_node(node.kind()) {
        let text = node_text(node, source);
        // Skip single-char identifiers and keywords
        if text.len() > 1 {
            refs.push(IdentRef {
                name: text.to_string(),
                line: node.start_position().row + 1,
                import_path: None, // resolved later
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_ident_refs(child, source, refs);
    }
}

fn is_identifier_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier"
            | "shorthand_field_identifier"
    )
}

/// Collect `use` declarations from Rust source.
/// Handles: `use foo::bar::Baz;`, `use foo::bar::{Baz, Qux};`, `use foo::bar as Alias;`
fn collect_rust_imports(node: Node, source: &str, imports: &mut Vec<ImportEntry>, file_end: usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "use_declaration" => {
                let scope_end = find_scope_end(child, file_end);
                let text = node_text(child, source);
                parse_rust_use(text, child.start_position().row + 1, scope_end, imports);
            }
            // Recurse into blocks/functions to find scoped imports
            "function_item" | "block" | "impl_item" | "mod_item" => {
                let block_end = child.end_position().row + 1;
                collect_rust_imports_scoped(child, source, imports, block_end);
            }
            _ => {}
        }
    }
}

fn collect_rust_imports_scoped(
    node: Node,
    source: &str,
    imports: &mut Vec<ImportEntry>,
    scope_end: usize,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "use_declaration" => {
                let text = node_text(child, source);
                parse_rust_use(text, child.start_position().row + 1, scope_end, imports);
            }
            "block" | "declaration_list" => {
                collect_rust_imports_scoped(child, source, imports, scope_end);
            }
            _ => {}
        }
    }
}

/// Parse a Rust `use` statement text into ImportEntry items.
fn parse_rust_use(text: &str, line: usize, scope_end: usize, imports: &mut Vec<ImportEntry>) {
    // Strip `use ` prefix and `;` suffix
    let text = text.trim();
    let text = text.strip_prefix("use ").unwrap_or(text);
    let text = text.strip_suffix(';').unwrap_or(text).trim();
    // Strip visibility (pub, pub(crate), etc.)
    let text = if text.starts_with("pub") {
        if let Some(rest) = text.strip_prefix("pub(") {
            // pub(crate) use ..., pub(super) use ...
            if let Some(after) = rest.find(')') {
                rest[after + 1..].trim()
            } else {
                text
            }
        } else {
            text.strip_prefix("pub ").unwrap_or(text).trim()
        }
    } else {
        text
    };

    // Handle `use foo::bar::{A, B, C};`
    if let Some(brace_start) = text.find('{') {
        let prefix = &text[..brace_start];
        let brace_end = text.rfind('}').unwrap_or(text.len());
        let inner = &text[brace_start + 1..brace_end];
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            // Handle `Foo as Bar`
            let (orig, alias) = if let Some(as_pos) = item.find(" as ") {
                (&item[..as_pos], &item[as_pos + 4..])
            } else {
                (item, item)
            };
            let alias = alias.trim();
            let full = format!("{}{}", prefix, orig.trim());
            if !alias.is_empty() && alias != "self" && alias != "*" {
                imports.push(ImportEntry {
                    name: alias.to_string(),
                    full_path: full,
                    line,
                    scope_end,
                });
            }
        }
    } else {
        // Simple: `use foo::bar::Baz;` or `use foo::bar::Baz as Alias;`
        let (path, alias) = if let Some(as_pos) = text.find(" as ") {
            (&text[..as_pos], &text[as_pos + 4..])
        } else {
            let name = text.rsplit("::").next().unwrap_or(text);
            (text, name)
        };
        let alias = alias.trim();
        if !alias.is_empty() && alias != "self" && alias != "*" {
            imports.push(ImportEntry {
                name: alias.to_string(),
                full_path: path.trim().to_string(),
                line,
                scope_end,
            });
        }
    }
}

/// Collect `import` declarations from TypeScript/Svelte.
fn collect_ts_imports(node: Node, source: &str, imports: &mut Vec<ImportEntry>, file_end: usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_statement" {
            let text = node_text(child, source);
            parse_ts_import(text, child.start_position().row + 1, file_end, imports);
        }
    }
}

/// Parse a TS `import { A, B } from 'module'` into ImportEntry items.
fn parse_ts_import(text: &str, line: usize, scope_end: usize, imports: &mut Vec<ImportEntry>) {
    // Extract module path from `from '...'` or `from "..."`
    let module = if let Some(from_pos) = text.find("from ") {
        let rest = &text[from_pos + 5..];
        rest.trim()
            .trim_matches(|c| c == '\'' || c == '"' || c == ';')
            .to_string()
    } else {
        return;
    };

    // Extract imported names from `{ A, B as C }`
    if let Some(brace_start) = text.find('{') {
        let brace_end = text.find('}').unwrap_or(text.len());
        let inner = &text[brace_start + 1..brace_end];
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (orig, alias) = if let Some(as_pos) = item.find(" as ") {
                (&item[..as_pos], &item[as_pos + 4..])
            } else {
                (item, item)
            };
            let alias = alias.trim();
            if !alias.is_empty() {
                imports.push(ImportEntry {
                    name: alias.to_string(),
                    full_path: format!("{}.{}", module, orig.trim()),
                    line,
                    scope_end,
                });
            }
        }
    }
    // Default import: `import Foo from '...'`
    else {
        let text_trimmed = text.trim().strip_prefix("import ").unwrap_or("");
        if let Some(name_end) = text_trimmed.find(|c: char| c.is_whitespace()) {
            let name = &text_trimmed[..name_end];
            if !name.is_empty() && name != "type" && !name.starts_with('{') {
                imports.push(ImportEntry {
                    name: name.to_string(),
                    full_path: format!("{}.default", module),
                    line,
                    scope_end,
                });
            }
        }
    }
}

/// Find the end line of the enclosing scope for a node.
fn find_scope_end(node: Node, file_end: usize) -> usize {
    let mut parent = node.parent();
    while let Some(p) = parent {
        match p.kind() {
            "block" | "declaration_list" | "function_item" => {
                return p.end_position().row + 1;
            }
            _ => parent = p.parent(),
        }
    }
    file_end
}

/// Resolve import paths for identifier refs.
fn resolve_refs(refs: &mut [IdentRef], imports: &[ImportEntry]) {
    for r in refs.iter_mut() {
        // Find the best matching import: same name, declared before the ref, ref within scope
        let best = imports
            .iter()
            .filter(|imp| imp.name == r.name && imp.line <= r.line && r.line <= imp.scope_end)
            .last(); // last = most recently declared (innermost scope)
        if let Some(imp) = best {
            r.import_path = Some(imp.full_path.clone());
        }
    }
}

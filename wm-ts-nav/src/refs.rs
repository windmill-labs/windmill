use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::Path;
use tree_sitter::{Node, Parser};

use crate::parser::Lang;

pub struct Ref {
    pub path: String,
    pub line: usize,
    pub context: String,
}

/// Find references to `name` across all files under `root`.
/// Uses tree-sitter to only match identifiers in code, skipping comments and strings.
pub fn find_refs(root: &Path, name: &str, limit: usize) -> Result<Vec<Ref>> {
    let files: Vec<_> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| Lang::from_path(e.path()).is_some())
        .map(|e| e.into_path())
        .collect();

    let name_owned = name.to_string();
    let results: Vec<Vec<Ref>> = files
        .par_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path).ok()?;
            // Quick text check — skip files that don't contain the name at all
            if !source.contains(&name_owned) {
                return None;
            }
            let refs = find_refs_in_file(path, &source, &name_owned).ok()?;
            if refs.is_empty() {
                None
            } else {
                Some(refs)
            }
        })
        .collect();

    let mut all: Vec<Ref> = results.into_iter().flatten().collect();
    all.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    all.truncate(limit);
    Ok(all)
}

fn find_refs_in_file(path: &Path, source: &str, name: &str) -> Result<Vec<Ref>> {
    let lang = Lang::from_path(path).unwrap();
    let code = match lang {
        Lang::Typescript if path.extension().map(|e| e == "svelte").unwrap_or(false) => {
            extract_svelte_script(source)
        }
        _ => source.to_string(),
    };

    let mut parser = Parser::new();
    match lang {
        Lang::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into())?,
        Lang::Typescript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?
        }
    }

    let tree = match parser.parse(&code, None) {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let lines: Vec<&str> = code.lines().collect();
    let mut refs = Vec::new();
    collect_identifier_refs(tree.root_node(), &code, name, &lines, path, &mut refs);
    Ok(refs)
}

fn collect_identifier_refs(
    node: Node,
    source: &str,
    name: &str,
    lines: &[&str],
    path: &Path,
    refs: &mut Vec<Ref>,
) {
    // Skip comment and string nodes entirely
    match node.kind() {
        "line_comment" | "block_comment" | "string_literal" | "raw_string_literal"
        | "string" | "template_string" | "string_fragment" | "comment" => return,
        _ => {}
    }

    // Check if this is an identifier node matching our name
    if is_identifier_node(node.kind()) {
        let text = &source[node.byte_range()];
        if text == name {
            let line = node.start_position().row;
            let context = lines
                .get(line)
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            refs.push(Ref {
                path: path.to_string_lossy().to_string(),
                line: line + 1,
                context,
            });
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifier_refs(child, source, name, lines, path, refs);
    }
}

fn is_identifier_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "field_identifier"
            | "scoped_identifier"
            | "property_identifier"
            | "shorthand_field_identifier"
    )
}

fn extract_svelte_script(source: &str) -> String {
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

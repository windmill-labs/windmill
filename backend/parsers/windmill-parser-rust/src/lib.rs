use anyhow::anyhow;
use regex::Regex;
use tree_sitter::Parser;
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn otyp_to_string(otyp: Option<String>) -> String {
    otyp.unwrap()
}

pub fn parse_rust_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language())?;
    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Parsed empty tree"))?;
    let mut cursor = tree.walk();
    let x = if let Some(main_fn) = tree.root_node().children(&mut cursor).find_map(|x| {
        if x.kind() == "function_item"
            && x.child_by_field_name("name")
                .map(|f| f.utf8_text(code.as_bytes()).map(|n| n == "main"))
                .unwrap_or(Ok(false))
                .unwrap_or(false)
        {
            Some(x)
        } else {
            None
        }
    }) {
        let mut cursor = main_fn.walk();
        let args = main_fn
            .child_by_field_name("parameters")
            .ok_or(anyhow!("No parameters on main function"))?
            .named_children(&mut cursor)
            .map(|param| {
                let param_name = param
                    .child_by_field_name("pattern")
                    .ok_or(anyhow!("No 'pattern' in node"))?
                    .utf8_text(code.as_bytes())?;
                let param_type = param
                    .child_by_field_name("type")
                    .ok_or(anyhow!("No 'pattern' in node"))?
                    .utf8_text(code.as_bytes())?;
                let (otyp, typ) = parse_rust_typ(param_type);
                Ok(Arg {
                    name: param_name.to_string(),
                    otyp,
                    typ,
                    default: None,
                    has_default: false,
                    oidx: None,
                })
            })
            // .take_while(Result::is_ok)
            .collect::<anyhow::Result<Vec<Arg>>>()?;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            no_main_func: Some(false),
        })
    } else {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(true),
        })
    };
    x
}

pub fn parse_rust_deps_into_manifest(code: &str) -> anyhow::Result<String> {
    const MODIFIABLE_MANIFEST_TABLES: &[&str] = &["dependencies"];

    let partial_manifest = find_embedded_manifest(code).unwrap_or(Manifest::Toml("".to_string()));
    let partial_manifest = partial_manifest.into_toml()?;
    let mut manif = default_manifest();
    for table_name in MODIFIABLE_MANIFEST_TABLES {
        match partial_manifest.get(*table_name) {
            Some(toml::Value::Table(tab)) => {
                // Merge.
                match manif.entry(*table_name) {
                    toml::map::Entry::Vacant(e) => {
                        e.insert(toml::Value::Table(tab.to_owned()));
                    }
                    toml::map::Entry::Occupied(e) => {
                        let into_t = match e.into_mut() {
                            toml::Value::Table(t) => Some(t),
                            _ => None,
                        };
                        into_t.ok_or(anyhow!(""))?.extend(tab.to_owned());
                    }
                }
            }
            Some(v) => {
                // Just replace.
                manif.insert(table_name.to_string(), v.to_owned());
            }
            None => (),
        }
    }

    Ok(manif.to_string())
}

fn parse_rust_typ(param_typ: &str) -> (Option<String>, Typ) {
    (
        Some(param_typ.to_string()),
        match param_typ {
            "usize" | "u8" | "u16" | "u32" | "u64" | "isize" | "i8" | "i16" | "i32" | "i64" => {
                Typ::Int
            }
            // "&str" | "String" => Typ::Str(),
            _ => Typ::Unknown,
        },
    )
}

#[derive(Debug, PartialEq)]
enum Manifest {
    Toml(String),
    DepList(String),
}

impl Manifest {
    pub fn into_toml(self) -> anyhow::Result<toml::value::Table> {
        match self {
            Manifest::Toml(s) => toml::from_str(&s),
            Manifest::DepList(s) => Manifest::dep_list_to_toml(&s),
        }
        .map_err(|e| anyhow!("Could not parse embedded manifest: {}", e))
    }

    fn dep_list_to_toml(s: &str) -> ::std::result::Result<toml::value::Table, toml::de::Error> {
        let mut r = String::new();
        r.push_str("[dependencies]\n");
        for dep in s.trim().split(',') {
            // If there's no version specified, add one.
            match dep.contains('=') {
                true => {
                    r.push_str(dep);
                    r.push('\n');
                }
                false => {
                    r.push_str(dep);
                    r.push_str("=\"*\"\n");
                }
            }
        }

        toml::from_str(&r)
    }
}

fn default_manifest() -> toml::value::Table {
    toml::from_str(include_str!("../manifest/Cargo.toml.default"))
        .expect("Failed to parse Cargo.toml.default")
}

/**
* Get a `Manifest` that can be made into a TOML table. The format and logic are the same as in
* the rust-script or cargo-eval projects.
*/
fn find_embedded_manifest(s: &str) -> Option<Manifest> {
    find_short_hand_manifest(s).or_else(|| find_code_block_manifest(s))
}

fn find_short_hand_manifest(s: &str) -> Option<Manifest> {
    let re: Regex = Regex::new(r"^(?i)\s*//\s*cargo-deps\s*:(.*?)(\r\n|\n)").unwrap();
    /*
    This is pretty simple: the only valid syntax for this is for the first, non-blank line to contain a single-line comment whose first token is `cargo-deps:`.  That's it.
    */
    if let Some(cap) = re.captures(s) {
        if let Some(m) = cap.get(1) {
            return Some(Manifest::DepList(m.as_str().to_string()));
        }
    }
    None
}

/**
Locates a "code block manifest" in Rust source.
*/
fn find_code_block_manifest(s: &str) -> Option<Manifest> {
    let re_crate_comment: Regex = {
        Regex::new(
            r"(?x)
                ^\s*
                (/\*!|//(!|/))
            ",
        )
        .unwrap()
    };

    let start = match re_crate_comment.captures(s) {
        Some(cap) => match cap.get(1) {
            Some(m) => m.start(),
            None => return None,
        },
        None => return None,
    };

    let comment = match extract_comment(&s[start..]) {
        Some(s) => s,
        None => {
            return None;
        }
    };

    scrape_markdown_manifest(&comment).map(|s| Manifest::Toml(s))
}

/**
Extracts the first `Cargo` fenced code block from a chunk of Markdown.
*/
fn scrape_markdown_manifest(content: &str) -> Option<String> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

    // To match librustdoc/html/markdown.rs, opts.
    let exts = Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES;

    let md = Parser::new_ext(content, exts);

    let mut found = false;
    let mut output = None;

    for item in md {
        match item {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
                if info.to_lowercase() == "cargo" && output.is_none() =>
            {
                found = true;
            }
            Event::Text(ref text) if found => {
                let s = output.get_or_insert(String::new());
                s.push_str(text);
            }
            Event::End(Tag::CodeBlock(_)) if found => {
                found = false;
            }
            _ => (),
        }
    }

    output
}

/**
Extracts the contents of a Rust doc comment.
*/
fn extract_comment(s: &str) -> Option<String> {
    use std::cmp::min;

    fn extract_block(s: &str) -> Option<String> {
        /*
        On every line:

        - update nesting level and detect end-of-comment
        - if margin is None:
            - if there appears to be a margin, set margin.
        - strip off margin marker
        - update the leading space counter
        - strip leading space
        - append content
        */
        let mut r = String::new();

        let margin_re: Regex = Regex::new(r"^\s*\*( |$)").unwrap();
        let space_re: Regex = Regex::new(r"^(\s+)").unwrap();
        let nesting_re: Regex = Regex::new(r"/\*|\*/").unwrap();

        let mut leading_space = None;
        let mut margin = None;
        let mut depth: u32 = 1;

        for line in s.lines() {
            if depth == 0 {
                break;
            }

            let mut end_of_comment = None;

            for (end, marker) in nesting_re.find_iter(line).map(|m| (m.start(), m.as_str())) {
                match (marker, depth) {
                    ("/*", _) => depth += 1,
                    ("*/", 1) => {
                        end_of_comment = Some(end);
                        depth = 0;
                        break;
                    }
                    ("*/", _) => depth -= 1,
                    _ => panic!("got a comment marker other than /* or */"),
                }
            }

            let line = end_of_comment.map(|end| &line[..end]).unwrap_or(line);

            margin = margin.or_else(|| margin_re.find(line).map(|m| m.as_str()));

            let line = if let Some(margin) = margin {
                let end = line
                    .char_indices()
                    .take(margin.len())
                    .map(|(i, c)| i + c.len_utf8())
                    .last()
                    .unwrap_or(0);
                &line[end..]
            } else {
                line
            };

            leading_space = leading_space.or_else(|| space_re.find(line).map(|m| m.end()));

                       let strip_len = min(leading_space.unwrap_or(0), line.len());
            let line = &line[strip_len..];

            r.push_str(line);

            r.push('\n');
        }

        Some(r)
    }

    fn extract_line(s: &str) -> Option<String> {
        let mut r = String::new();

        let comment_re = Regex::new(r"^\s*//(!|/)").unwrap();

        let space_re = Regex::new(r"^(\s+)").unwrap();

        let mut leading_space = None;

        for line in s.lines() {
            let content = match comment_re.find(line) {
                Some(m) => &line[m.end()..],
                None => break,
            };

            leading_space = leading_space.or_else(|| {
                space_re
                    .captures(content)
                    .and_then(|c| c.get(1))
                    .map(|m| m.end())
            });

            let strip_len = min(leading_space.unwrap_or(0), content.len());
            let content = &content[strip_len..];

            r.push_str(content);

            r.push('\n');
        }

        Some(r)
    }

    if let Some(stripped) = s.strip_prefix("/*!") {
        extract_block(stripped)
    } else if s.starts_with("//!") || s.starts_with("///") {
        extract_line(s)
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_rust_signature() {
        let code = r#"
// commenting comments

fn main(my_int: i8) -> Result<String, String> {
    println!("My int is {}", my_int);
}"#;

        let ret = parse_rust_signature(code).unwrap();

        assert_eq!(ret.args.len(), 1);
        assert_eq!(ret.args[0].name, "my_int");
        assert_eq!(ret.args[0].typ, Typ::Int);
        assert_eq!(ret.args[0].otyp, Some("i8".to_string()));
    }

    #[test]
    fn test_parse_rust_manifest() {
        assert_eq!(find_embedded_manifest("fn main() {}"), None);

        let code = r#"

//! Crate doc comment right here
//! We need to use the `cargo` language in the code bloc
//!
//! ```cargo
//! [dependencies]
//! time = "0.1.25"
//! ```

fn main() {
    println!("{}", time::now().rfc822z());
}
        "#;
        let manif = find_embedded_manifest(code);

        assert_eq!(
            manif,
            Some(Manifest::Toml(
                r#"[dependencies]
time = "0.1.25"
"#
                .to_string()
            ))
        );
        let _full_manif = parse_rust_deps_into_manifest(code).unwrap();
    }

    #[test]
    fn test_default_manifest() {
        default_manifest();
    }
}

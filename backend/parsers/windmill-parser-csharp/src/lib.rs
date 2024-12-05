#![feature(c_variadic)]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::anyhow;
use tree_sitter::Node;
use windmill_parser::Arg;
use windmill_parser::MainArgSignature;
use windmill_parser::Typ;

pub fn parse_csharp_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting c# as language: {e}"))?;

    // Parse code
    let tree = parser.parse(code, None).expect("Failed to parse code");
    let root_node = tree.root_node();

    // Traverse the AST to find the Main method signature
    let main_sig = find_main_signature(root_node, code);
    let no_main_func = Some(main_sig.is_none());

    let mut args = vec![];
    if let Some(sig) = main_sig {
        // for (i, c) in sig.children(&mut sig.walk()).enumerate() {
        //     println!("  {:?} - {:?}", c, sig.field_name_for_child((i) as u32));
        // }
        if let Some(param_list) = sig.child_by_field_name("parameters") {
            for c in param_list.children(&mut param_list.walk()) {
                if c.kind() == "parameter" {
                    let (otyp, typ, name) = parse_csharp_typ(c, code);
                    args.push(Arg {
                        name,
                        otyp,
                        typ,
                        default: None,
                        has_default: false,
                        oidx: None,
                    });
                    for (i, w) in c.children(&mut c.walk()).enumerate() {
                        let s = w.utf8_text(code.as_bytes());
                        println!(
                            "  {:?} - {:?} - {:?}",
                            w,
                            c.field_name_for_child((i) as u32),
                            s
                        );
                    }
                }
            }
        } else {
            println!("No one with parameter_list");
        }
    }

    Ok(MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args,
        has_preprocessor: None,
        no_main_func,
    })
}

fn parse_csharp_typ<'a>(param_node: Node<'a>, code: &str) -> (Option<String>, Typ, String) {
    let name = param_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .unwrap_or("");
    let otyp_node = param_node.child_by_field_name("type");
    let otyp = otyp_node
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .map(|s| s.to_string());

    let typ = Typ::Str(None);

    (otyp, typ, name.to_string())
}

// Function to find the Main method's signature
fn find_main_signature<'a>(root_node: Node<'a>, code: &str) -> Option<Node<'a>> {
    let mut cursor = root_node.walk();
    for x in root_node.children(&mut cursor) {
        if x.kind() == "class_declaration" {
            for c in x.children(&mut x.walk()) {
                if c.kind() == "declaration_list" {
                    for w in c.children(&mut c.walk()) {
                        if w.kind() == "method_declaration" {
                            for child in w.children(&mut w.walk()) {
                                if child
                                    .utf8_text(code.as_bytes())
                                    .map(|name| name == "Main")
                                    .unwrap_or(false)
                                {
                                    return Some(w);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return None;
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_parse_csharp_sig() {
        let code = r#"
using System;

class LilProgram
{

    public static string Main(string myString = "World", int myInt)
    {
        Console.Writeline("Hello!!");
        return "yeah";
    }

}"#;
        let ret = parse_csharp_signature(code).unwrap();

        assert_eq!(ret.args.len(), 2);

        assert_eq!(ret.args[0].name, "myString");
        assert_eq!(ret.args[0].otyp, Some("string".to_string()));
        assert_eq!(ret.args[0].typ, Typ::Str(None));

        assert_eq!(ret.args[1].name, "myInt");
        assert_eq!(ret.args[1].otyp, Some("int".to_string()));
        assert_eq!(ret.args[1].typ, Typ::Int);
    }
}

#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::anyhow;
use tree_sitter::Node;
use windmill_parser::Arg;
use windmill_parser::MainArgSignature;
use windmill_parser::{ObjectType, Typ};

#[derive(Debug)]
pub struct CsharpMainSigMeta {
    pub is_async: bool,
    pub is_public: bool,
    pub returns_void: bool,
    pub class_name: Option<String>,
    pub main_sig: MainArgSignature,
}

fn csharp_param_default_value<'a>(def: Node<'a>, code: &str) -> Option<serde_json::Value> {
    def.utf8_text(code.as_bytes())
        .ok()
        .and_then(|content| serde_json::from_str(content).ok())
}

pub fn parse_csharp_sig_meta(code: &str) -> anyhow::Result<CsharpMainSigMeta> {
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
    let mut is_async = false;
    let mut is_public = false;
    let mut returns_void = false;
    let mut class_name = None;

    let mut args = vec![];
    if let Some((sig, name)) = main_sig {
        class_name = name;
        for sig_node in sig.children(&mut sig.walk()) {
            if sig_node.kind() == "modifier" && sig_node.utf8_text(code.as_bytes())? == "async" {
                is_async = true;
            }
            if sig_node.kind() == "modifier" && sig_node.utf8_text(code.as_bytes())? == "public" {
                is_public = true;
            }
        }
        if let Some(return_type) = sig.child_by_field_name("returns") {
            let return_type = return_type.utf8_text(code.as_bytes())?;

            if return_type == "void" || (is_async && return_type == "Task") {
                returns_void = true;
            }
        }
        if let Some(param_list) = sig.child_by_field_name("parameters") {
            for p_list_node in param_list.children(&mut param_list.walk()) {
                if p_list_node.kind() == "parameter" {
                    let mut default = None;
                    for a in p_list_node.children(&mut p_list_node.walk()) {
                        if a.kind() == "=" {
                            if let Some(node) = a.next_sibling() {
                                default = csharp_param_default_value(node, code);
                            }
                        }
                    }
                    let (otyp, typ, name) = parse_csharp_typ(p_list_node, code)?;
                    args.push(Arg { name, otyp, typ, default, has_default: false, oidx: None });
                }
            }
        }
    }

    let main_sig = MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args,
        has_preprocessor: None,
        no_main_func,
    };

    Ok(CsharpMainSigMeta { is_async, returns_void, class_name, main_sig, is_public })
}

pub fn parse_csharp_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    Ok(parse_csharp_sig_meta(code)?.main_sig)
}

fn find_typ<'a>(typ_node: Node<'a>, code: &str) -> anyhow::Result<Typ> {
    match typ_node.kind() {
        "predefined_type" => {
            match typ_node.utf8_text(code.as_bytes()) {
                Ok("string") => Ok(Typ::Str(None)),
                Ok("sbyte") | Ok("System.SByte") => Ok(Typ::Bytes),
                Ok("byte") | Ok("System.Byte") => Ok(Typ::Bytes),
                Ok("short") | Ok("System.Int16") => Ok(Typ::Int),
                Ok("ushort") | Ok("System.UInt16") => Ok(Typ::Int),
                Ok("int") | Ok("System.Int32") => Ok(Typ::Int),
                Ok("uint") | Ok("System.UInt32") => Ok(Typ::Int),
                Ok("long") | Ok("System.Int64") => Ok(Typ::Int),
                Ok("ulong") | Ok("System.UInt64") => Ok(Typ::Int),
                Ok("char") | Ok("System.Char") => Ok(Typ::Str(None)),
                Ok("float") | Ok("System.Single") => Ok(Typ::Float),
                Ok("double") | Ok("System.Double") => Ok(Typ::Float),
                Ok("bool") | Ok("System.Boolean") => Ok(Typ::Bool),
                Ok("decimal") | Ok("System.Decimal") => Ok(Typ::Float),
                Ok("object") => Ok(Typ::Object(ObjectType::new(None, Some(vec![])))), // TODO: Complete the object type
                Ok(s) => Err(anyhow!("Unknown type `{s}`")),
                Err(e) => Err(anyhow!("Error getting type name: {}", e)),
            }
        }
        "array_type" => {
            let new_typ_node = typ_node
                .child_by_field_name("type")
                .ok_or(anyhow!("Failed to find inner type of array type"))?;
            Ok(Typ::List(Box::new(find_typ(new_typ_node, code)?)))
        }
        "identifier" => Ok(Typ::Unknown),
        "generic_name" => Ok(Typ::Unknown),
        "pointer_type" => Ok(Typ::Int),
        "nullable_type" => {
            let new_typ_node = typ_node
                .child_by_field_name("type")
                .ok_or(anyhow!("Failed to find inner type of nullable_type"))?;
            Ok(find_typ(new_typ_node, code)?)
        }
        wc => Err(anyhow!(
            "Unexpected C# type node kind: {} for '{}'. This type is not handeled by Windmill, please open an issue if this seems to be an error",
            wc,
            typ_node.utf8_text(code.as_bytes())?
        )),
    }
}

fn parse_csharp_typ<'a>(
    param_node: Node<'a>,
    code: &str,
) -> anyhow::Result<(Option<String>, Typ, String)> {
    let name = param_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .unwrap_or("");
    let otyp_node = param_node.child_by_field_name("type");
    let otyp = otyp_node
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .map(|s| s.to_string());

    let typ = find_typ(otyp_node.unwrap(), code)?;

    Ok((otyp, typ, name.to_string()))
}

// Function to find the Main method's signature
fn find_main_signature<'a>(root_node: Node<'a>, code: &str) -> Option<(Node<'a>, Option<String>)> {
    let mut cursor = root_node.walk();
    for x in root_node.children(&mut cursor) {
        if x.kind() == "class_declaration" {
            let class_name = x
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(code.as_bytes()).ok().map(|s| s.to_string()));
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
                                    return Some((w, class_name));
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

pub fn parse_csharp_reqs(code: &str) -> (Vec<(String, Option<String>)>, Vec<usize>) {
    let mut nuget_reqs = Vec::new();
    let mut pkg_lines = Vec::new();

    for (i, line) in code.split("\n").enumerate() {
        if line.starts_with('#') {
            if let Some(req) = parse_nuget_req(&line) {
                pkg_lines.push(i);
                nuget_reqs.push(req);
            }
        } else {
            break; // Stop processing after the first non-comment line
        }
    }

    (nuget_reqs, pkg_lines)
}

fn parse_nuget_req(line: &str) -> Option<(String, Option<String>)> {
    // Check if the line starts with `#r "nuget:`
    if let Some(start) = line.find("#r \"nuget:") {
        // Extract the content after `#r "nuget:`
        let start_idx = start + 10;
        let end_idx = line[start_idx..].find('"')?;
        let line = &line[start_idx..start_idx + end_idx];
        let mut splitted = line.split(",");
        if let Some(pkg) = splitted.next() {
            return Some((
                pkg.trim().to_string(),
                splitted.next().map(|s| s.trim().to_string()),
            ));
        }
    }
    None
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_parse_csharp_sig_and_meta() {
        let code = r#"
using System;

class LilProgram
{

    public async static string Main(string myString = "World", int myInt = 2, string[] jj = ["asd", "ss"])
    {
        Console.Writeline("Hello!!");
        return "yeah";
    }

}"#;
        let sig_meta = parse_csharp_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("LilProgram".to_string()));
        assert_eq!(sig_meta.is_async, true);

        let ret = sig_meta.main_sig;
        assert_eq!(ret.args.len(), 3);
    }

    #[test]
    fn test_parse_csharp_sig() {
        let code = r#"
using System;

class LilProgram
{

    public static string Main(string myString = "World", int myInt, string[] jj)
    {
        Console.Writeline("Hello!!");
        return "yeah";
    }

}"#;
        let ret = parse_csharp_signature(code).unwrap();

        assert_eq!(ret.args.len(), 3);

        assert_eq!(ret.args[0].name, "myString");
        assert_eq!(ret.args[0].otyp, Some("string".to_string()));
        assert_eq!(ret.args[0].typ, Typ::Str(None));

        assert_eq!(ret.args[1].name, "myInt");
        assert_eq!(ret.args[1].otyp, Some("int".to_string()));
        assert_eq!(ret.args[1].typ, Typ::Int);

        assert_eq!(ret.args[2].name, "jj");
        assert_eq!(ret.args[2].otyp, Some("string[]".to_string()));
        assert_eq!(ret.args[2].typ, Typ::List(Box::new(Typ::Str(None))));
    }

    #[test]
    fn test_parse_csharp_reqs() {
        let file_content = r#"#r "nuget: AutoMapper, 6.1.0"
#r "nuget: Newtonsoft.Json, 13.0.1"
#r "nuget: Serilog, 2.10.0"

#r "nuget: Serilog, 2.10.0"

using System;
"#;

        let requirements = parse_csharp_reqs(file_content).0;

        assert_eq!(requirements.len(), 3);
        assert_eq!(
            requirements[0],
            ("AutoMapper".to_string(), Some("6.1.0".to_string()))
        );
        assert_eq!(
            requirements[1],
            ("Newtonsoft.Json".to_string(), Some("13.0.1".to_string()))
        );
        assert_eq!(
            requirements[2],
            ("Serilog".to_string(), Some("2.10.0".to_string()))
        );
    }
}

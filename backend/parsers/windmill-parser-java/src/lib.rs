#![cfg_attr(target_arch = "wasm32", feature(c_variadic))]

#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

use anyhow::anyhow;
use anyhow::bail;
use serde_json::Value;
use tree_sitter::Node;
use windmill_parser::Arg;
use windmill_parser::MainArgSignature;
use windmill_parser::{ObjectType, Typ};

#[derive(Debug)]
pub struct JavaMainSigMeta {
    pub is_public: bool,
    pub returns_void: bool,
    pub class_name: Option<String>,
    pub main_sig: MainArgSignature,
}

pub fn parse_java_sig_meta(code: &str) -> anyhow::Result<JavaMainSigMeta> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| anyhow!("Error setting Java as language: {e}"))?;

    // Parse code
    let tree = parser
        .parse(code, None)
        .ok_or(anyhow!("Failed to parse code"))?;
    let root_node = tree.root_node();

    // Traverse the AST to find the Main method signature
    let main_sig = find_main_signature(root_node, code);
    let no_main_func = Some(main_sig.is_none());
    let mut is_public = false;
    let mut returns_void = false;
    let mut class_name = None;

    let mut args = vec![];
    if let Some((sig, name)) = main_sig {
        class_name = name;
        for sig_node in sig.children(&mut sig.walk()) {
            if sig_node.kind() == "modifier" && sig_node.utf8_text(code.as_bytes())? == "public" {
                is_public = true;
            }
        }
        if let Some(return_type) = sig.child_by_field_name("type") {
            let return_type = return_type.utf8_text(code.as_bytes())?;

            if return_type == "void" {
                returns_void = true;
            }
        }
        if let Some(param_list) = sig.child_by_field_name("parameters") {
            for p_list_node in param_list.children(&mut param_list.walk()) {
                if p_list_node.kind() == "formal_parameter" {
                    let (otyp, typ, name, default) = parse_java_typ(p_list_node, code)?;
                    args.push(Arg {
                        name,
                        otyp,
                        typ,
                        has_default: default.is_some(),
                        default,
                        oidx: None,
                    });
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

    Ok(JavaMainSigMeta { returns_void, class_name, main_sig, is_public })
}

pub fn parse_java_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    Ok(parse_java_sig_meta(code)?.main_sig)
}

fn find_typ<'a>(typ_node: Node<'a>, code: &str) -> anyhow::Result<(Typ, Option<Value>)> {
    let null = Some(serde_json::Value::Null);
    let res = match typ_node.kind() {
        #[rustfmt::skip]
        "type_identifier" => {
            match typ_node.utf8_text(code.as_bytes()) {
                Ok("String")      => (Typ::Str(None), null),
                Ok("Byte")        => (Typ::Bytes, null),
                Ok("Short")       => (Typ::Int, null),
                Ok("Integer")     => (Typ::Int, null),
                Ok("Long")        => (Typ::Int, null),
                Ok("Float")       => (Typ::Float, null),
                Ok("Double")      => (Typ::Float, null),
                Ok("Boolean")     => (Typ::Bool, null),
                Ok("Character")   => (Typ::Str(None), null),
                Ok("Object")      => (Typ::Object(ObjectType::new(None, Some(vec![]))),null), // TODO: Complete the object type
                Ok(s)       => bail!("Unknown type `{s}`"),
                Err(e) => bail!("Error getting type name: {}", e),
            }
        }
        #[rustfmt::skip]
        "integral_type" => {
            match typ_node.utf8_text(code.as_bytes()) {
                Ok("byte")   => (Typ::Bytes, None),
                Ok("short")  => (Typ::Int, None),
                Ok("int")    => (Typ::Int, None),
                Ok("long")   => (Typ::Int, None),
                Ok("char")   => (Typ::Str(None), None),
                Ok(s)  => bail!("Unknown type `{s}`"),
                Err(e) => bail!("Error getting type name: {}", e),
            }
        }
        "floating_point_type" => {
            match typ_node.utf8_text(code.as_bytes()) {
                Ok("float") => (Typ::Float, None),
                Ok("double") => (Typ::Float, None),
                Ok(s) => bail!("Unknown type `{s}`"),
                Err(e) => bail!("Error getting type name: {}", e),
            }
        }
        "boolean_type" => {
            match typ_node.utf8_text(code.as_bytes()) {
                Ok("boolean") => (Typ::Bool, None),
                Ok(s) => bail!("Unknown type `{s}`"),
                Err(e) => bail!("Error getting type name: {}", e),
            }
        }
        "array_type" => {
            let new_typ_node = typ_node
                .named_child(0)
                .ok_or(anyhow!("Failed to find inner type of array type"))?;
            (Typ::List(Box::new(find_typ(new_typ_node, code)?.0)), null)
        }
        wc => bail!(
            "Unexpected Java type node kind: {} for '{}'. This type is not handled by Windmill, please open an issue if this seems to be an error",
            wc,
            typ_node.utf8_text(code.as_bytes())?
        ),

    };
    Ok(res)
}

fn parse_java_typ<'a>(
    param_node: Node<'a>,
    code: &str,
) -> anyhow::Result<(Option<String>, Typ, String, Option<Value>)> {
    let name = param_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .unwrap_or("");
    let otyp_node = param_node.child_by_field_name("type");
    let otyp = otyp_node
        .and_then(|n| n.utf8_text(code.as_bytes()).ok())
        .map(|s| s.to_string());

    let (typ, default) = find_typ(
        otyp_node.ok_or(anyhow!(
            "Internal error: Failed to get child by field name 'type'"
        ))?,
        code,
    )?;

    Ok((otyp, typ, name.to_string(), default))
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
                if c.kind() == "class_body" {
                    for w in c.children(&mut c.walk()) {
                        if w.kind() == "method_declaration" {
                            for child in w.children(&mut w.walk()) {
                                if child
                                    .utf8_text(code.as_bytes())
                                    .map(|name| name == "main")
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

#[cfg(test)]
mod test {

    use serde_json::json;

    use super::*;
    #[test]
    fn test_parse_java_return_void() {
        let code = r#"
class Main {
    public static void main() {}
}"#;
        let sig_meta = parse_java_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("Main".to_string()));

        assert!(sig_meta.returns_void);
    }
    #[test]
    fn test_parse_java_return_object() {
        let code = r#"
class Main {
    public static Object main() {}
}"#;
        let sig_meta = parse_java_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("Main".to_string()));
        assert!(!sig_meta.returns_void);
    }
    #[test]
    fn test_parse_java_primitive_types() {
        let code = r#"
class Main {
    public static string main(byte a, short b, int c, long d, float e, double f, boolean g, char h) {}
}"#;
        let sig_meta = parse_java_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("Main".to_string()));

        let ret = sig_meta.main_sig;
        assert_eq!(
            ret.args,
            vec![
                Arg {
                    name: "a".into(),
                    otyp: Some("byte".into()),
                    typ: Typ::Bytes,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "b".into(),
                    otyp: Some("short".into()),
                    typ: Typ::Int,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "c".into(),
                    otyp: Some("int".into()),
                    typ: Typ::Int,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "d".into(),
                    otyp: Some("long".into()),
                    typ: Typ::Int,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "e".into(),
                    otyp: Some("float".into()),
                    typ: Typ::Float,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "f".into(),
                    otyp: Some("double".into()),
                    typ: Typ::Float,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "g".into(),
                    otyp: Some("boolean".into()),
                    typ: Typ::Bool,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "h".into(),
                    otyp: Some("char".into()),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: None
                },
            ]
        );
    }

    #[test]
    fn test_parse_java_objects() {
        let code = r#"
class Main {
    public static string main(Byte a, Short b, Integer c, Long d, Float e, Double f, Boolean g, Character h, Object i) {}
}"#;
        let sig_meta = parse_java_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("Main".to_string()));

        let ret = sig_meta.main_sig;
        assert_eq!(
            ret.args,
            vec![
                Arg {
                    name: "a".into(),
                    otyp: Some("Byte".into()),
                    typ: Typ::Bytes,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "b".into(),
                    otyp: Some("Short".into()),
                    typ: Typ::Int,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "c".into(),
                    otyp: Some("Integer".into()),
                    typ: Typ::Int,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "d".into(),
                    otyp: Some("Long".into()),
                    typ: Typ::Int,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "e".into(),
                    otyp: Some("Float".into()),
                    typ: Typ::Float,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "f".into(),
                    otyp: Some("Double".into()),
                    typ: Typ::Float,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "g".into(),
                    otyp: Some("Boolean".into()),
                    typ: Typ::Bool,
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "h".into(),
                    otyp: Some("Character".into()),
                    typ: Typ::Str(None),
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "i".into(),
                    otyp: Some("Object".into()),
                    typ: Typ::Object(ObjectType::new(None, Some(vec![]))),
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
            ]
        );
    }
    #[test]
    fn test_parse_java_array() {
        let code = r#"
class Main {
    public static string main(int[] a, Object[] b, String[] c) {}
}"#;
        let sig_meta = parse_java_sig_meta(code).unwrap();

        assert_eq!(sig_meta.class_name, Some("Main".to_string()));

        let ret = sig_meta.main_sig;
        assert_eq!(
            ret.args,
            vec![
                Arg {
                    name: "a".into(),
                    otyp: Some("int[]".into()),
                    typ: Typ::List(Box::new(Typ::Int)),
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "b".into(),
                    otyp: Some("Object[]".into()),
                    typ: Typ::List(Box::new(Typ::Object(ObjectType::new(None, Some(vec![]))))),
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    name: "c".into(),
                    otyp: Some("String[]".into()),
                    typ: Typ::List(Box::new(Typ::Str(None))),
                    default: Some(json!(null)),
                    has_default: true,
                    oidx: None
                },
            ]
        );
    }
}

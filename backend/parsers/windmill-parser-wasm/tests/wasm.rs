use serde_json::json;
use wasm_bindgen_test::wasm_bindgen_test;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};
use windmill_parser_ts::parse_deno_signature;

#[wasm_bindgen_test]
fn test_parse_deno_sig() -> anyhow::Result<()> {
    let code = "
export function main(test1?: string, test2: string = \"burkina\",
    test3: wmill.Resource<'postgres'>, b64: Base64, ls: Base64[], 
    email: Email, literal: \"test\", literal_union: \"test\" | \"test2\",
    opt_type?: string | null, opt_type_union: string | null, opt_type_union_union2: string | undefined,
    min_object: {a: string, b: number}) {
    console.log(42)
}
";
    assert_eq!(
        parse_deno_signature(code, false)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    otyp: None,
                    name: "test1".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "test2".to_string(),
                    typ: Typ::Str(None),
                    default: Some(json!("burkina")),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "test3".to_string(),
                    typ: Typ::Resource("postgres".to_string()),
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "b64".to_string(),
                    typ: Typ::Bytes,
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "ls".to_string(),
                    typ: Typ::List(Box::new(Typ::Bytes)),
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "email".to_string(),
                    typ: Typ::Email,
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "literal".to_string(),
                    typ: Typ::Str(Some(vec!["test".to_string()])),
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "literal_union".to_string(),
                    typ: Typ::Str(Some(vec!["test".to_string(), "test2".to_string()])),
                    default: None,
                    has_default: false
                },
                Arg {
                    otyp: None,
                    name: "opt_type".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "opt_type_union".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "opt_type_union_union2".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "min_object".to_string(),
                    typ: Typ::Object(vec![
                        ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                        ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Float) }
                    ]),
                    default: None,
                    has_default: false
                }
            ]
        }
    );

    Ok(())
}

#[wasm_bindgen_test]
fn test_parse_deno_sig_implicit_types() -> anyhow::Result<()> {
    let code = "
export function main(test2 = \"burkina\",
    bool = true,
    float = 4.2,
    int = 42,
    ls = [\"test\"],
    min_object = {a: \"test\", b: 42}) {
    console.log(42)
}
";
    assert_eq!(
        parse_deno_signature(code, false)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    otyp: None,
                    name: "test2".to_string(),
                    typ: Typ::Str(None),
                    default: Some(json!("burkina")),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "bool".to_string(),
                    typ: Typ::Bool,
                    default: Some(json!(true)),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "float".to_string(),
                    typ: Typ::Float,
                    default: Some(json!(4.2)),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "int".to_string(),
                    typ: Typ::Int,
                    default: Some(json!(42)),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "ls".to_string(),
                    typ: Typ::List(Box::new(Typ::Str(None))),
                    default: Some(json!(["test"])),
                    has_default: true
                },
                Arg {
                    otyp: None,
                    name: "min_object".to_string(),
                    typ: Typ::Object(vec![
                        ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                        ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Int) }
                    ]),
                    default: Some(json!({"a": "test", "b": 42})),
                    has_default: true
                }
            ]
        }
    );

    Ok(())
}

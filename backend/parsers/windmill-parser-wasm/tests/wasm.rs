use serde_json::json;
use wasm_bindgen_test::wasm_bindgen_test;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};
use windmill_parser_bash::parse_powershell_sig;
use windmill_parser_ts::{parse_deno_signature, parse_expr_for_ids, parse_expr_for_imports};

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_deno_sig() -> anyhow::Result<()> {
    let code = "
export function main(test1?: string, test2: string = \"burkina\",
    test3: wmill.Resource<'postgres'>, b64: Base64, ls: Base64[], 
    email: Email, literal: \"test\", literal_union: \"test\" | \"test2\",
    opt_type?: string | null, opt_type_union: string | null, opt_type_union_union2: string | undefined,
    min_object: {a: string, b: number},
    literals_with_undefined: \"foo\" | \"bar\" | undefined,
    dyn_select: DynSelect_foo) {
    console.log(42)
}
";
    assert_eq!(
        parse_deno_signature(code, false, false, None)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    otyp: None,
                    name: "test1".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test2".to_string(),
                    typ: Typ::Str(None),
                    default: Some(json!("burkina")),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test3".to_string(),
                    typ: Typ::Resource("postgres".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "b64".to_string(),
                    typ: Typ::Bytes,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "ls".to_string(),
                    typ: Typ::List(Box::new(Typ::Bytes)),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "email".to_string(),
                    typ: Typ::Email,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "literal".to_string(),
                    typ: Typ::Str(Some(vec!["test".to_string()])),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "literal_union".to_string(),
                    typ: Typ::Str(Some(vec!["test".to_string(), "test2".to_string()])),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "opt_type".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "opt_type_union".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "opt_type_union_union2".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "min_object".to_string(),
                    typ: Typ::Object(vec![
                        ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                        ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Float) }
                    ]),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "literals_with_undefined".to_string(),
                    typ: Typ::Str(Some(vec!["foo".to_string(), "bar".to_string()])),
                    default: None,
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "dyn_select".to_string(),
                    typ: Typ::DynSelect("foo".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None
                }
            ],
            no_main_func: Some(false),
            has_preprocessor: Some(false)
        }
    );

    Ok(())
}

#[allow(dead_code)]
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
        parse_deno_signature(code, false, false, None)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    otyp: None,
                    name: "test2".to_string(),
                    typ: Typ::Str(None),
                    default: Some(json!("burkina")),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "bool".to_string(),
                    typ: Typ::Bool,
                    default: Some(json!(true)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "float".to_string(),
                    typ: Typ::Float,
                    default: Some(json!(4.2)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "int".to_string(),
                    typ: Typ::Int,
                    default: Some(json!(42)),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "ls".to_string(),
                    typ: Typ::List(Box::new(Typ::Str(None))),
                    default: Some(json!(["test"])),
                    has_default: true,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "min_object".to_string(),
                    typ: Typ::Object(vec![
                        ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                        ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Int) }
                    ]),
                    default: Some(json!({"a": "test", "b": 42})),
                    has_default: true,
                    oidx: None
                }
            ],
            no_main_func: Some(false),
            has_preprocessor: Some(false)
        }
    );

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_deno_types() -> anyhow::Result<()> {
    let code = "
type FooBar = {
    a: string,
    b: number,
}
export function main(foo: FooBar, {a, b}: FooBar, {c, d}: FooBar = {a: \"foo\", b: 42}) {

}
";
    assert_eq!(
        parse_deno_signature(code, false, false, None)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    name: "foo".to_string(),
                    otyp: None,
                    typ: Typ::Resource("foo_bar".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "anon1".to_string(),
                    otyp: None,
                    typ: Typ::Resource("foo_bar".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    name: "anon2".to_string(),
                    otyp: None,
                    typ: Typ::Resource("foo_bar".to_string()),
                    default: Some(json!({"a": "foo", "b": 42})),
                    has_default: true,
                    oidx: None
                }
            ],
            no_main_func: Some(false),
            has_preprocessor: Some(false)
        }
    );

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_enum_list() -> anyhow::Result<()> {
    let code = "
export function main(foo: (\"foo\" | \"bar\")[]) {

}
";
    assert_eq!(
        parse_deno_signature(code, false, false, None)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![Arg {
                name: "foo".to_string(),
                otyp: None,
                typ: Typ::List(Box::new(Typ::Str(Some(vec![
                    "foo".to_string(),
                    "bar".to_string()
                ])))),
                default: None,
                has_default: false,
                oidx: None
            }],
            no_main_func: Some(false),
            has_preprocessor: Some(false)
        }
    );

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_extract_ident() -> anyhow::Result<()> {
    let code = "
    let foo = 3;
    bar
    baroof.foob.ar
    foobar[barfoo.x]
";
    assert_eq!(
        parse_expr_for_ids(code)?,
        vec![
            ("baroof".to_string(), "foob".to_string()),
            ("barfoo".to_string(), "x".to_string())
        ]
    );

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_imports() -> anyhow::Result<()> {
    let code = "
    import * as foo from '@foo/bar';
    import { bar } from \"./bar\";
    import { bar } from \"bar/foo/d\";
    import { bar as baroof } from \"bar\";
";
    let mut l = parse_expr_for_imports(code)?;
    l.sort();
    assert_eq!(
        l,
        vec![
            "./bar".to_string(),
            "@foo/bar".to_string(),
            "bar".to_string(),
            "bar/foo/d".to_string()
        ]
    );

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_imports_dts() -> anyhow::Result<()> {
    let code = "
export type foo = number
";
    let mut l = parse_expr_for_imports(code)?;
    l.sort();
    assert_eq!(l, vec![] as Vec<String>);

    Ok(())
}

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_parse_powershell_sig() -> anyhow::Result<()> {
    let code = "
param($test_none, [string]$test_string [int]$test_int, [decimal]$test_decimal, [double]$test_double, [single]$test_single, [datetime]$test_datetime_lower, [DateTime]$test_datetime_upper)

Write-Output 'Testing...'
";
    assert_eq!(
        parse_powershell_sig(code)?,
        MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![
                Arg {
                    otyp: None,
                    name: "test_none".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_string".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_int".to_string(),
                    typ: Typ::Int,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_decimal".to_string(),
                    typ: Typ::Float,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_double".to_string(),
                    typ: Typ::Float,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_single".to_string(),
                    typ: Typ::Float,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_datetime_lower".to_string(),
                    typ: Typ::Datetime,
                    default: None,
                    has_default: false,
                    oidx: None
                },
                Arg {
                    otyp: None,
                    name: "test_datetime_upper".to_string(),
                    typ: Typ::Datetime,
                    default: None,
                    has_default: false,
                    oidx: None
                }
            ],
            no_main_func: None,
            has_preprocessor: None
        }
    );

    Ok(())
}

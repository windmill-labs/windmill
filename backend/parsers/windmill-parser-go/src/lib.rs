#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use gosyn::{
    ast::{Declaration, Expression, Field, Ident, StructType},
    parse_source,
};
use itertools::Itertools;

use regex::Regex;
use windmill_parser::{to_snake_case, Arg, MainArgSignature, ObjectProperty, Typ};

lazy_static::lazy_static! {
    pub static ref REQUIRE_PARSE: Regex = Regex::new(r"//require (.*)\n").unwrap();
}

pub fn parse_go_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let filtered_code = filter_non_main(code);
    let file = parse_source(&filtered_code).map_err(|x| anyhow::anyhow!(x.to_string()))?;
    if let Some(func) = file.decl.iter().find_map(|x| match x {
        Declaration::Function(func) if &func.name.name == "main" => Some(func),
        _ => None,
    }) {
        let args = func
            .typ
            .params
            .list
            .iter()
            .map(|param| {
                let (otyp, typ) = parse_go_typ(&param.typ);
                Arg {
                    name: get_name(param),
                    otyp,
                    typ,
                    default: None,
                    has_default: false,
                    oidx: None,
                }
            })
            .collect_vec();
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            no_main_func: Some(false),
            has_preprocessor: None,
        })
    } else {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(true),
            has_preprocessor: None,
        })
    }
}

pub fn parse_go_imports(code: &str) -> anyhow::Result<Vec<String>> {
    let file =
        parse_source(filter_non_imports(code)).map_err(|x| anyhow::anyhow!(x.to_string()))?;
    let mut imports: Vec<String> = file
        .imports
        .iter()
        .filter_map(|x| {
            if x.path.value.contains("/") {
                Some(x.path.value.clone())
            } else {
                None
            }
        })
        .collect();
    imports.sort();
    Ok([
        imports,
        REQUIRE_PARSE
            .captures_iter(code)
            .map(|x| x[1].to_string())
            .collect_vec(),
    ]
    .concat())
}

fn get_name(param: &Field) -> String {
    param
        .name
        .first()
        .map(|y| y.name.to_string())
        .unwrap_or_else(|| "".to_string())
}

fn parse_go_typ(typ: &Expression) -> (Option<String>, Typ) {
    match typ {
        Expression::Ident(Ident { name, .. }) => (
            Some((name).to_string()),
            match name.as_str() {
                "int" => Typ::Int,
                "int16" => Typ::Int,
                "int32" => Typ::Int,
                "int64" => Typ::Int,
                "string" => Typ::Str(None),
                "bool" => Typ::Bool,
                _ => Typ::Resource(to_snake_case(name.as_ref())),
            },
        ),
        Expression::TypeSlice(slice_type) => {
            let (inner_otyp, inner_typ) = parse_go_typ(&*slice_type.typ);
            (
                inner_otyp.map(|x| format!("[]{}", x)),
                Typ::List(Box::new(inner_typ)),
            )
        }
        Expression::TypeArray(array_type) => {
            let (inner_otyp, inner_typ) = parse_go_typ(&*array_type.typ);
            (
                inner_otyp.map(|x| format!("[]{x}")),
                Typ::List(Box::new(inner_typ)),
            )
        }
        Expression::TypeStruct(StructType { fields, .. }) => {
            let (otyps, typs): (Vec<String>, Vec<ObjectProperty>) = fields
                .iter()
                .map(|field| {
                    let json_tag = field
                        .tag
                        .as_ref()
                        .and_then(|x| x.value.strip_prefix("`json:\""))
                        .and_then(|x| x.strip_suffix("\"`"))
                        .and_then(|x| x.split(',').next().map(|x| x.to_string()));
                    let (otyp, typ) = parse_go_typ(&field.typ);
                    let name = get_name(field);
                    let key = json_tag.unwrap_or_else(|| name.to_string());
                    (
                        format!("{name} {} `json:\"{key}\"`", otyp_to_string(otyp)),
                        ObjectProperty { key, typ: Box::new(typ) },
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .unzip();
            (
                Some(format!(
                    "struct {{ {} }}",
                    otyps.iter().join("; ").to_string()
                )),
                Typ::Object(typs),
            )
        }
        Expression::TypeInterface(_) => (Some("interface{}".to_string()), Typ::Object(vec![])),
        Expression::TypeMap(_) => (
            Some("map[string]interface{}".to_string()),
            Typ::Object(vec![]),
        ),
        _ => (None, Typ::Unknown),
    }
}

pub fn otyp_to_string(otyp: Option<String>) -> String {
    otyp.unwrap_or_else(|| "interface{}".to_string())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_go_sig_resources() -> anyhow::Result<()> {
        let code = r#"

package main

import "fmt"

type MyResource struct {
	A string   `json:"a"`
	B *int64   `json:"b,omitempty"`
	C []string `json:"c,omitempty"`
}

func main(x int, a MyResource) {
    fmt.Println("hello world")
}    

"#;
        assert_eq!(
            parse_go_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "x".to_string(),
                        typ: Typ::Int,
                        has_default: false,
                        default: None,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("MyResource".to_string()),
                        name: "a".to_string(),
                        typ: Typ::Resource("my_resource".into()),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None
            }
        );

        Ok(())
    }
    #[test]
    fn test_parse_go_sig() -> anyhow::Result<()> {
        let code = r#"

package main

import "fmt"

func main(x int, y string, z bool, l []string, o struct { Name string `json:"name"` }, n interface{}, m map[string]interface{}) {
    fmt.Println("hello world")
}    

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_go_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "x".to_string(),
                        typ: Typ::Int,
                        has_default: false,
                        default: None,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("string".to_string()),
                        name: "y".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("bool".to_string()),
                        name: "z".to_string(),
                        typ: Typ::Bool,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("[]string".to_string()),
                        name: "l".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("struct { Name string `json:\"name\"` }".to_string()),
                        name: "o".to_string(),
                        typ: Typ::Object(vec![ObjectProperty {
                            key: "name".to_string(),
                            typ: Box::new(Typ::Str(None))
                        },]),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("interface{}".to_string()),
                        name: "n".to_string(),
                        typ: Typ::Object(vec![]),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("map[string]interface{}".to_string()),
                        name: "m".to_string(),
                        typ: Typ::Object(vec![]),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None
            }
        );

        Ok(())
    }
}

#[test]
fn test_parse_go_import() -> anyhow::Result<()> {
    let code = r#"
package inner

import (
    "fmt"
    "rsc.io/quote"
    wmill "github.com/windmill-labs/windmill-go-client"
)

// the main must return (interface{}, error)

func main(x string, nested struct {
    Foo string `json:"foo"`
}) (interface{}, error) {
    fmt.Println("Hello, World")
    fmt.Println(nested.Foo)
    fmt.Println(quote.Opt())
    v, _ := wmill.GetVariable("f/examples/secret")
    return v, nil
}
"#;

    assert_eq!(
        parse_go_imports(code)?,
        vec![
            "\"github.com/windmill-labs/windmill-go-client\"",
            "\"rsc.io/quote\""
        ]
    );

    Ok(())
}

fn filter_non_imports(code: &str) -> String {
    code.split_once("func ")
        .map(|(x, _)| x.to_string())
        .unwrap_or_else(|| code.to_string())
}
fn filter_non_main(code: &str) -> String {
    const FUNC_MAIN: &str = "func main(";

    let mut filtered_code = "package main;\n".to_string();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(FUNC_MAIN) {
            filtered_code += FUNC_MAIN;
            remaining += line.strip_prefix(FUNC_MAIN).unwrap();
            remaining += &code_iter.join("\n");
            break;
        }
    }
    if filtered_code.is_empty() {
        return String::new();
    }
    let mut chars = remaining.chars();
    let mut open_parens = 1;

    while let Some(c) = chars.next() {
        if c == '(' {
            open_parens += 1;
        } else if c == ')' {
            open_parens -= 1;
        }
        filtered_code.push(c);
        if open_parens == 0 {
            break;
        }
    }

    filtered_code.push_str("{}");
    return filtered_code;
}

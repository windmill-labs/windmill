/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use convert_case::{Boundary, Case, Casing};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug, PartialEq, Default)]
pub struct MainArgSignature {
    pub star_args: bool,
    pub star_kwargs: bool,
    pub args: Vec<Arg>,
    pub no_main_func: Option<bool>,
    pub has_preprocessor: Option<bool>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all(serialize = "lowercase"))]
pub struct ObjectProperty {
    pub key: String,
    pub typ: Box<Typ>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all(serialize = "lowercase"))]
pub struct OneOfVariant {
    pub label: String,
    pub properties: Vec<ObjectProperty>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum Typ {
    Str(Option<Vec<String>>),
    Int,
    Float,
    Bool,
    List(Box<Typ>),
    Bytes,
    Datetime,
    Resource(String),
    Email,
    Sql,
    DynSelect(String),
    Object(Vec<ObjectProperty>),
    OneOf(Vec<OneOfVariant>),
    Unknown,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: String,
    pub otyp: Option<String>,
    pub typ: Typ,
    pub default: Option<serde_json::Value>,
    pub has_default: bool,
    pub oidx: Option<i32>,
}

pub fn json_to_typ(js: &Value) -> Typ {
    match js {
        Value::String(_) => Typ::Str(None),
        Value::Number(n) if n.is_i64() => Typ::Int,
        Value::Number(_) => Typ::Float,
        Value::Bool(_) => Typ::Bool,
        Value::Object(o) => Typ::Object(
            o.iter()
                .map(|(k, v)| ObjectProperty { key: k.to_string(), typ: Box::new(json_to_typ(v)) })
                .collect(),
        ),
        Value::Array(a) => Typ::List(Box::new(a.first().map(json_to_typ).unwrap_or(Typ::Unknown))),
        _ => Typ::Unknown,
    }
}

pub fn to_snake_case(s: &str) -> String {
    s.with_boundaries(&Boundary::defaults())
        .without_boundaries(&Boundary::letter_digit())
        .to_case(Case::Snake)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_snake_case() {
        assert_eq!("s3", to_snake_case("S3"));
        assert_eq!("s3", to_snake_case("s3"));
        assert_eq!("s_3", to_snake_case("S_3"));
        assert_eq!("type_name_here", to_snake_case("typeNameHere"));
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_single_char_lowercase() {
        assert_eq!(to_snake_case("a"), "a");
    }

    #[test]
    fn test_single_char_uppercase() {
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_all_uppercase() {
        assert_eq!(to_snake_case("TEST"), "test");
    }

    #[test]
    fn test_all_lowercase() {
        assert_eq!(to_snake_case("test"), "test");
    }

    #[test]
    fn test_mixed_case() {
        assert_eq!(to_snake_case("testCase"), "test_case");
    }

    #[test]
    fn test_mixed_case_with_numbers() {
        assert_eq!(to_snake_case("testCase1"), "test_case1");
        assert_eq!(to_snake_case("Test123Case"), "test123_case");
    }

    #[test]
    fn test_numbers_with_hyphen() {
        assert_eq!(to_snake_case("test-3"), "test_3");
    }

    #[test]
    fn test_string_with_spaces() {
        assert_eq!(to_snake_case("This is a Test"), "this_is_a_test");
    }

    #[test]
    fn test_snake_case_input() {
        assert_eq!(to_snake_case("already_snake_case"), "already_snake_case");
    }

    #[test]
    fn test_kebab_case_input() {
        assert_eq!(to_snake_case("already-kebab-case"), "already_kebab_case");
    }

    #[test]
    fn test_mixed_delimiters() {
        assert_eq!(
            to_snake_case("test-Case_with Spaces"),
            "test_case_with_spaces"
        );
    }

    #[test]
    fn test_leading_and_trailing_spaces() {
        assert_eq!(to_snake_case("  test case  "), "test_case");
    }
}

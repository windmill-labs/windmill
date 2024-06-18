/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug, PartialEq)]
pub struct MainArgSignature {
    pub star_args: bool,
    pub star_kwargs: bool,
    pub args: Vec<Arg>,
    pub no_main_func: Option<bool>,
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

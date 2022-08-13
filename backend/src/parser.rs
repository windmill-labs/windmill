/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::Serialize;

#[derive(Serialize)]
pub struct MainArgSignature {
    pub star_args: bool,
    pub star_kwargs: bool,
    pub args: Vec<Arg>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all(serialize = "lowercase"))]
pub struct ObjectProperty {
    pub key: String,
    pub typ: Box<Typ>,
}

#[derive(Serialize, Clone)]
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
    Unknown,
}

#[derive(Serialize, Clone)]
pub struct Arg {
    pub name: String,
    pub typ: Typ,
    pub default: Option<serde_json::Value>,
    pub has_default: bool,
}

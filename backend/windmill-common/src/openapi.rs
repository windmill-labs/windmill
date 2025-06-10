use std::collections::HashMap;

use crate::{error::Result, worker::to_raw_value};
use anyhow::anyhow;
use axum::http;
use http::Method as HttpMethod;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use url::Url;

lazy_static::lazy_static! {
    static ref DEFAULT_OPENAPI_INFO_OBJECT: Info = Info {
        title: "Windmill API".to_string(),
        version: "1.0.0".to_string(),
        ..Default::default()
    };
}

const DEFAULT_OPENAPI_GENERATED_VERSION: &'static str = "3.1.0";

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum Format {
    JSON,
    YAML
}

impl Default for Format {
    fn default() -> Self {
        Self::YAML
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Contact {
    name: String,
    url: String,
    email: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct License {
    name: String,
    identifier: Option<String>,
    url: Option<Url>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Info {
    title: String,
    version: String,
    description: Option<String>,
    terms_of_service: Option<String>,
    contact: Option<Contact>,
    license: Option<License>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    url: Url,
    description: Option<String>,
    variables: Option<HashMap<String, Value>>,
}

pub struct FuturePath {
    route_path: String,
    http_method: HttpMethod,
    is_async: bool,
}

impl FuturePath {
    pub fn new(route_path: String, http_method: HttpMethod, is_async: bool) -> FuturePath {
        FuturePath { route_path, http_method, is_async }
    }
}

fn from_route_path_to_openapi_path(route_path: &str) -> Result<(String, Option<Box<RawValue>>)> {
    let mut openapi_path = String::new();
    let mut parameters = Vec::new();

    for segment in route_path.split('/') {
        if segment.starts_with(':') {
            let param_name = &segment[1..];

            if param_name.is_empty() {
                return Err(anyhow!("Empty parameter name in path: {}", route_path).into());
            }

            openapi_path.push_str(&format!("/{{{}}}", param_name));
            parameters.push(serde_json::json!({
                "name": param_name,
                "in": "path",
                "required": true,
                "schema": { "type": "string" }
            }));
        } else if !segment.is_empty() {
            openapi_path.push('/');
            openapi_path.push_str(segment);
        } else {
            openapi_path.push('/');
        }
    }

    let parameters_json = if parameters.is_empty() {
        None
    } else {
        Some(to_raw_value(&parameters))
    };

    let normalized_path = if route_path.starts_with('/') {
        route_path.to_string()
    } else {
        format!("/{}", route_path)
    };

    Ok((normalized_path, parameters_json))
}

fn generate_paths(paths: Vec<FuturePath>) -> Result<HashMap<String, Value>> {
    let mut map: IndexMap<String, HashMap<String, Box<RawValue>>> = IndexMap::new();

    for path in paths {
        let (route_path, parameters) = from_route_path_to_openapi_path(&path.route_path)?;
        let http_method = path.http_method.to_string().to_lowercase();

        match map.get_mut(&route_path) {
            Some(paths) => {
                let path_object = paths.get(&http_method);

                if path_object.is_some() {
                    return Err(anyhow!("Found duplicate route: {}", path.route_path).into());
                }

                let responses = if path.is_async {
                    serde_json::json!({
                        "200": {
                            "description": "Returns a job ID as a UUID string.",
                            "content": {
                                "text/plain": {
                                    "schema": {
                                        "type": "string",
                                        "format": "uuid",
                                        "example": "550e8400-e29b-41d4-a716-446655440000"
                                    }
                                }
                            }
                        }
                    })
                } else {
                    serde_json::json!({
                        "200": {
                            "description": "Response.",
                            "content": {
                                "*/*": {
                                    "schema": {}
                                }
                            }
                        }
                    })
                };

                let request_response_body = serde_json::json!({
                    "requestBody": {
                        "description": "The request body",
                        "content": {
                            "application/json": { "schema": {} },
                            "application/cloudevents+json": { "schema": {} },
                            "text/plain": { "schema": {} },
                            "application/x-www-form-urlencoded": { "schema": {} },
                            "application/xml": { "schema": {} },
                            "text/xml": { "schema": {} },
                            "multipart/form-data": { "schema": {} }
                        }
                    },
                    "responses": responses
                });

                paths.insert(http_method, to_raw_value(&request_response_body));
            }
            None => {
                map.insert(
                    route_path,
                    HashMap::from([("parameters".to_string(), to_raw_value(&parameters))]),
                );
            }
        }
    }

    return Ok(HashMap::new());
}

pub fn generate_openapi_document(
    info: Option<&Info>,
    url: Option<&Url>,
    paths: Option<Vec<FuturePath>>,
    format: Format
) -> Result<String> {
    let servers = url.map_or_else(
        || vec![],
        |url| vec![Server { url: url.to_owned(), description: None, variables: None }],
    );

    let paths = paths.map(generate_paths).transpose()?;

    let mut openapi_doc: IndexMap<&'static str, Box<RawValue>> = IndexMap::new();

    openapi_doc.insert("openapi", to_raw_value(&DEFAULT_OPENAPI_GENERATED_VERSION));
    openapi_doc.insert(
        "info",
        to_raw_value(info.unwrap_or(&DEFAULT_OPENAPI_INFO_OBJECT)),
    );

    if !servers.is_empty() {
        openapi_doc.insert("servers", to_raw_value(&servers));
    }

    if let Some(paths) = paths {
        openapi_doc.insert("paths", to_raw_value(&paths));
    }

    let pretiffied_json = serde_json::to_string_pretty(&openapi_doc)
        .map_err(|err| anyhow!("Could not generate OpenAPI document: {}", err))?;

    Ok(pretiffied_json)
}

use std::collections::HashMap;

use crate::{error::Result, utils::RunnableKind, worker::to_raw_value};
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
    YAML,
}

impl Default for Format {
    fn default() -> Self {
        Self::YAML
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct License {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Info {
    title: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terms_of_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<License>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<HashMap<String, Value>>,
}

pub struct FuturePath {
    route_path: String,
    http_method: HttpMethod,
    is_async: bool,
    runnable_kind: Option<RunnableKind>,
}

impl FuturePath {
    pub fn new(
        route_path: String,
        http_method: HttpMethod,
        is_async: bool,
        runnable_kind: Option<RunnableKind>,
    ) -> FuturePath {
        FuturePath { route_path, http_method, is_async, runnable_kind }
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

    let normalized_path = if openapi_path.starts_with('/') {
        openapi_path
    } else {
        format!("/{}", openapi_path)
    };

    Ok((normalized_path, parameters_json))
}

fn generate_paths(
    paths: Vec<FuturePath>,
    url: Option<&Url>,
) -> Result<IndexMap<String, IndexMap<String, Box<RawValue>>>> {
    let mut map: IndexMap<String, IndexMap<String, Box<RawValue>>> = IndexMap::new();

    let generate_default_request = || {
        serde_json::json!({
            "description": "This route may or may not require a request body, but its structure and content type are unknown.",
            "required": false,
            "content": {}
        })
    };

    let generate_response = |is_async: bool| {
        let responses = if is_async {
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
            serde_json::json!(serde_json::json!({
                "200": {
                    "description": "This route may return a response, but its structure and content type are unknown.",
                    "content": {}
                }
            }))
        };

        responses
    };

    for path in paths {
        let (route_path, parameters) = from_route_path_to_openapi_path(&path.route_path)?;
        let http_method = path.http_method.to_string().to_lowercase();

        match map.get_mut(&route_path) {
            Some(paths) => {
                let path_object = paths.get(&http_method);

                if path_object.is_some() {
                    return Err(anyhow!("Found duplicate route: {}", path.route_path).into());
                }

                let mut request_response_map = IndexMap::with_capacity(2);

                if path.http_method != HttpMethod::GET {
                    request_response_map.insert("requestBody", generate_default_request());
                }

                request_response_map.insert("responses", generate_response(path.is_async));

                paths.insert(http_method, to_raw_value(&request_response_map));
            }
            None => {
                let mut path_object = IndexMap::new();
                if let Some(url) = url {
                    let url = url.as_str().trim_end_matches('/');

                    let server = match path.runnable_kind {
                        Some(RunnableKind::Script) => Server {
                            url: format!("{}/api/w/{{workspace}}/jobs/run/p", &url),
                            variables: Some(HashMap::from([(
                                "workspace".to_string(),
                                serde_json::json!({
                                    "default": "test",
                                    "description": "Workspace identifier"
                                }),
                            )])),
                            description: None,
                        },
                        Some(RunnableKind::Flow) => Server {
                            url: format!("{}/api/w/{{workspace}}/jobs/run/f", &url),
                            variables: Some(HashMap::from([(
                                "workspace".to_string(),
                                serde_json::json!({
                                    "default": "test",
                                    "description": "Workspace identifier"
                                }),
                            )])),
                            description: None,
                        },
                        None => Server {
                            url: format!("{}/api/r", url),
                            description: None,
                            variables: None,
                        },
                    };

                    path_object.insert("servers".to_string(), to_raw_value(&vec![server]));
                }

                if parameters.is_some() {
                    path_object.insert("parameters".to_string(), to_raw_value(&parameters));
                }

                let mut request_response_map = IndexMap::with_capacity(2);

                if path.http_method != HttpMethod::GET {
                    request_response_map.insert("requestBody", generate_default_request());
                }

                request_response_map.insert("responses", generate_response(path.is_async));

                path_object.insert(http_method, to_raw_value(&request_response_map));

                map.insert(route_path, path_object);
            }
        }
    }

    return Ok(map);
}

pub fn transform_to_minified_postgres_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    for ch in glob.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    regex
}

#[derive(Debug, Default)]
pub struct ServerToSet {
    pub http_route: bool,
    pub webhook_flow: bool,
    pub webhook_script: bool,
}

impl ServerToSet {
    pub fn new(http_route: bool, webhook_flow: bool, webhook_script: bool) -> ServerToSet {
        ServerToSet { http_route, webhook_flow, webhook_script }
    }
}

pub fn generate_openapi_document(
    info: Option<&Info>,
    url: Option<&Url>,
    paths: Vec<FuturePath>,
    format: Format,
) -> Result<String> {
    let paths = generate_paths(paths, url)?;

    let mut openapi_doc: IndexMap<&'static str, Box<RawValue>> = IndexMap::new();

    openapi_doc.insert("openapi", to_raw_value(&DEFAULT_OPENAPI_GENERATED_VERSION));
    openapi_doc.insert(
        "info",
        to_raw_value(info.unwrap_or(&DEFAULT_OPENAPI_INFO_OBJECT)),
    );

    openapi_doc.insert("paths", to_raw_value(&paths));

    let openapi_document = match format {
        Format::YAML => "".to_string(),
        Format::JSON => serde_json::to_string_pretty(&openapi_doc).map_err(|err| {
            anyhow!(
                "Could not generate OpenAPI document in JSON format: {}",
                err
            )
        })?,
    };

    Ok(openapi_document)
}

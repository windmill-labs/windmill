use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::{
    error::Result,
    utils::RunnableKind,
    utils::{deserialize_url, is_empty},
};
use anyhow::anyhow;
use axum::http;
use http::Method as HttpMethod;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Map, Value};
use url::Url;

lazy_static::lazy_static! {
    static ref DEFAULT_OPENAPI_INFO_OBJECT: Info = Info {
        title: "Windmill API".to_string(),
        version: "1.0.0".to_string(),
        ..Default::default()
    };
}

const DEFAULT_OPENAPI_GENERATED_VERSION: &'static str = "3.1.0";
const JWT_SECURITY_SCHEME: &'static str = "JwtAuth";
const BASIC_HTTP_AUTH_SCHEME: &'static str = "BasicHttp";

const DEFAULT_REQUEST_KEY: &'static str = "defaultRequest";
const DEFAULT_ASYNC_RESPONSE_KEY: &'static str = "AsyncResponse";
const DEFAULT_SYNC_RESPONSE_KEY: &'static str = "SyncResponse";
const DEFAULT_PAYLOAD_PARAM_KEY: &'static str = "PayloadParam";

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    JSON,
    YAML,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format = match self {
            Format::JSON => "json",
            Format::YAML => "yaml",
        };
        write!(f, "{}", format)
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::YAML
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Contact {
    #[serde(skip_serializing_if = "is_empty")]
    name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_url",
        skip_serializing_if = "Option::is_none"
    )]
    url: Option<Url>,
    #[serde(skip_serializing_if = "is_empty")]
    email: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct License {
    name: String,
    #[serde(skip_serializing_if = "is_empty")]
    identifier: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_url",
        skip_serializing_if = "Option::is_none"
    )]
    url: Option<Url>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Info {
    title: String,
    version: String,
    #[serde(skip_serializing_if = "is_empty")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<License>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    url: String,
    #[serde(skip_serializing_if = "is_empty")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<HashMap<String, Value>>,
}

#[derive(Debug)]
pub enum SecurityScheme {
    BearerJwt,
    BasicHttp,
    ApiKey(String),
}
#[derive(Debug)]
pub struct WebhookConfig {
    runnable_kind: RunnableKind,
}

impl WebhookConfig {
    pub fn new(runnable_kind: RunnableKind) -> Self {
        Self { runnable_kind }
    }
}

#[derive(Debug)]
pub struct HttpRouteConfig {
    method: HttpMethod,
}

impl HttpRouteConfig {
    pub fn new(method: HttpMethod) -> Self {
        Self { method }
    }
}

#[derive(Debug)]
pub enum Kind {
    Webhook(WebhookConfig),
    HttpRoute(HttpRouteConfig),
}

#[derive(Debug)]
pub struct FuturePath {
    route_path: String,
    kind: Kind,
    is_async: Option<bool>,
    summary: Option<String>,
    description: Option<String>,
    security_scheme: Option<SecurityScheme>,
}

impl FuturePath {
    pub fn new(
        route_path: String,
        kind: Kind,
        is_async: Option<bool>,
        summary: Option<String>,
        description: Option<String>,
        security_scheme: Option<SecurityScheme>,
    ) -> FuturePath {
        FuturePath { route_path, kind, is_async, summary, description, security_scheme }
    }
}

fn from_route_path_to_openapi_path(
    route_path: &str,
    kind: &Kind,
) -> Result<(Vec<String>, Option<Value>)> {
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
        Some(Value::Array(parameters))
    };

    let prefix = match kind {
        Kind::HttpRoute(_) => "",
        Kind::Webhook(WebhookConfig { runnable_kind }) => match runnable_kind {
            RunnableKind::Script => "p",
            RunnableKind::Flow => "f",
        },
    };

    let normalized_path = if openapi_path.starts_with('/') {
        format!("{prefix}{openapi_path}")
    } else {
        format!("{}/{}", prefix, openapi_path)
    };

    let route_paths = if prefix.is_empty() {
        vec![normalized_path]
    } else {
        vec![
            format!("/run/{}", &normalized_path),
            format!("/run_wait_result/{}", &normalized_path),
        ]
    };

    Ok((route_paths, parameters_json))
}

fn get_servers_component(url: &str, kind: &Kind) -> Server {
    let url = url.trim_end_matches('/');

    let server = match kind {
        Kind::HttpRoute(_) => {
            Server { url: format!("{}/api/r", url), description: None, variables: None }
        }
        Kind::Webhook(_) => Server {
            url: format!("{}/api/w/{{workspace}}/jobs", url),
            variables: Some(HashMap::from([(
                "workspace".to_string(),
                serde_json::json!({
                    "default": "test",
                    "description": "Workspace identifier"
                }),
            )])),
            description: None,
        },
    };

    server
}

fn generate_paths(
    paths: Vec<FuturePath>,
    url: Option<&Url>,
) -> Result<IndexMap<String, IndexMap<String, Value>>> {
    let mut map: IndexMap<String, IndexMap<String, Value>> = IndexMap::new();

    let generate_default_request = || {
        serde_json::json!({
            "$ref": format!("#/components/requestBodies/{DEFAULT_REQUEST_KEY}")
        })
    };

    let generate_response = |is_async: bool| {
        let responses = if is_async {
            serde_json::json!({
                "200": {
                    "$ref": format!("#/components/responses/{DEFAULT_ASYNC_RESPONSE_KEY}")
                }
            })
        } else {
            serde_json::json!(serde_json::json!({
                "200": {
                    "$ref": format!("#/components/responses/{DEFAULT_SYNC_RESPONSE_KEY}")
                }
            }))
        };

        responses
    };

    let get_security_scheme = |security_scheme: Option<&SecurityScheme>| -> Vec<Value> {
        if let Some(security_scheme) = security_scheme {
            let scheme = match security_scheme {
                SecurityScheme::ApiKey(api_key) => header_to_pascal_case(&api_key),
                SecurityScheme::BearerJwt => JWT_SECURITY_SCHEME.to_owned(),
                SecurityScheme::BasicHttp => BASIC_HTTP_AUTH_SCHEME.to_owned(),
            };

            vec![serde_json::json!({
                scheme: []
            })]
        } else {
            vec![]
        }
    };

    let mut duplicate_webhooks = HashSet::new();

    for path in paths {
        let (route_paths, parameters) =
            from_route_path_to_openapi_path(&path.route_path, &path.kind)?;

        for route_path in route_paths {
            let path_object = map.entry(route_path.clone()).or_insert_with(|| {
                let mut path_object = IndexMap::new();

                if let Some(url) = url {
                    let servers = get_servers_component(url.as_str(), &path.kind);
                    path_object.insert("servers".to_string(), to_value(vec![servers]).unwrap());
                }

                if parameters.is_some() {
                    path_object.insert(
                        "parameters".to_string(),
                        to_value(parameters.clone()).unwrap(),
                    );
                }

                path_object
            });

            let is_async;

            let (methods, is_webhook) = match &path.kind {
                Kind::Webhook(_) => {
                    if !duplicate_webhooks.insert(route_path.clone()) {
                        return Err(anyhow!("Found duplicate webhook: {}", path.route_path).into());
                    }
                    is_async = route_path.starts_with("/run/");
                    let methods = if is_async {
                        vec![HttpMethod::POST]
                    } else {
                        vec![HttpMethod::GET, HttpMethod::POST]
                    };

                    (methods, true)
                }
                Kind::HttpRoute(HttpRouteConfig { method }) => {
                    if path_object.get(&method.to_string()).is_some() {
                        return Err(anyhow!("Found duplicate route: {}", path.route_path).into());
                    }
                    is_async = path.is_async.unwrap_or(true);
                    (vec![method.to_owned()], false)
                }
            };

            for method in methods {
                let mut method_map = IndexMap::new();

                if let Some(summary) = path.summary.as_ref().filter(|s| !s.is_empty()) {
                    method_map.insert("summary", Value::String(summary.to_owned()));
                }

                if let Some(description) = path.description.as_ref().filter(|s| !s.is_empty()) {
                    method_map.insert("description", Value::String(description.to_owned()));
                }

                method_map.insert(
                    "security",
                    to_value(get_security_scheme(path.security_scheme.as_ref()))?,
                );

                if method != HttpMethod::GET {
                    method_map.insert("requestBody", generate_default_request());
                } else if is_webhook {
                    method_map.insert(
                        "parameters",
                        Value::Array(vec![serde_json::json!({
                            "$ref": format!("#/components/parameters/{DEFAULT_PAYLOAD_PARAM_KEY}")
                        })]),
                    );
                }

                method_map.insert("responses", generate_response(is_async));

                path_object.insert(method.to_string().to_lowercase(), to_value(&method_map)?);
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

fn header_to_pascal_case(header: &str) -> String {
    header
        .split(|c: char| c == '-' || c == '_' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string()
                        + chars.as_str().to_ascii_lowercase().as_str()
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

#[derive(Debug, Default)]
struct SecuritySchemeToAdd {
    basic_http: bool,
    bearer_jwt: bool,
    api_key: Option<Vec<(String, Value)>>,
}

fn generate_all_security_schemes(future_paths: &[FuturePath]) -> SecuritySchemeToAdd {
    let mut to_add = SecuritySchemeToAdd::default();

    let mut vec = Vec::new();
    let mut set = HashSet::new();
    for future_path in future_paths {
        if !to_add.basic_http
            && matches!(future_path.security_scheme, Some(SecurityScheme::BasicHttp))
        {
            to_add.basic_http = true
        } else if !to_add.bearer_jwt
            && matches!(future_path.security_scheme, Some(SecurityScheme::BearerJwt))
        {
            to_add.bearer_jwt = true
        } else if let Some(SecurityScheme::ApiKey(api_key)) = future_path.security_scheme.as_ref() {
            let pascal_case_header = header_to_pascal_case(&api_key);

            if !set.insert(pascal_case_header.clone()) {
                continue;
            }

            let scheme = serde_json::json!({
                    "type": "apiKey",
                    "in": "header",
                    "name": api_key
            });
            vec.push((pascal_case_header, scheme));
        }
    }

    if vec.len() > 0 {
        to_add.api_key = Some(vec);
    }

    to_add
}

fn generate_components(future_paths: &[FuturePath]) -> Map<String, Value> {
    let mut components = Map::new();

    if future_paths
        .iter()
        .any(|path| matches!(path.kind, Kind::Webhook(_)))
    {
        components.insert(
            "parameters".to_owned(),
            serde_json::json!({
                "PayloadParam": {
                    "name": "payload",
                    "in": "query",
                    "required": true,
                    "description": "A URL-safe base64-encoded JSON string payload.",
                    "schema": {
                        "type": "string"
                    }
                }
            }),
        );
    }

    {
        let mut security_scheme = Map::new();

        let SecuritySchemeToAdd { basic_http, bearer_jwt, api_key } =
            generate_all_security_schemes(future_paths);

        if basic_http {
            security_scheme.insert(
                BASIC_HTTP_AUTH_SCHEME.to_owned(),
                serde_json::json!({
                    "type": "http",
                    "scheme": "basic"
                }),
            );
        }

        if bearer_jwt {
            security_scheme.insert(
                JWT_SECURITY_SCHEME.to_owned(),
                serde_json::json!({
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }),
            );
        }

        if let Some(api_key) = api_key {
            for (key, value) in api_key {
                security_scheme.insert(key, value);
            }
        }
        components.insert("securitySchemes".to_owned(), Value::Object(security_scheme));
    }

    components.insert("requestBodies".to_owned(), serde_json::json!({
       DEFAULT_REQUEST_KEY: {
            "description": "This route may or may not require a request body, but its structure and content type are unknown.",
            "required": false,
            "content": {
                "application/json": {}
            }
        }
    }));

    components.insert("responses".to_owned(), serde_json::json!({
        DEFAULT_ASYNC_RESPONSE_KEY: {
            "description": "Returns a job ID as a UUID string.",
            "content": {
                "text/plain": {
                    "schema": {
                        "type": "string",
                        "format": "uuid",
                        "examples": [ "550e8400-e29b-41d4-a716-446655440000" ]
                    }
                }
            }
        },
        DEFAULT_SYNC_RESPONSE_KEY: {
            "description": "This route may return a response, but its structure and content type are unknown.",
            "content": {
                "application/octet-stream": {}
            }
        },

    }));

    components
}

pub fn generate_openapi_document(
    info: Option<&Info>,
    url: Option<&Url>,
    paths: Vec<FuturePath>,
    format: Format,
) -> Result<String> {
    let mut openapi_doc: IndexMap<&'static str, Value> = IndexMap::new();

    openapi_doc.insert("openapi", to_value(&DEFAULT_OPENAPI_GENERATED_VERSION)?);
    openapi_doc.insert(
        "info",
        to_value(info.unwrap_or(&DEFAULT_OPENAPI_INFO_OBJECT))?,
    );

    openapi_doc.insert("components", Value::Object(generate_components(&paths)));

    openapi_doc.insert("paths", to_value(generate_paths(paths, url)?)?);

    let openapi_document = match format {
        Format::YAML => serde_yml::to_string(&openapi_doc).map_err(|err| {
            anyhow!(
                "Could not generate OpenAPI document in YAML format: {}",
                err
            )
        })?,
        Format::JSON => serde_json::to_string_pretty(&openapi_doc).map_err(|err| {
            anyhow!(
                "Could not generate OpenAPI document in JSON format: {}",
                err
            )
        })?,
    };

    Ok(openapi_document)
}

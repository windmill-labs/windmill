use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use anyhow::anyhow;
use axum::{
    body::Body, extract::Path, http, response::Response, routing::post, Extension, Json, Router,
};
use http::{header, HeaderValue, Method, StatusCode};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Map, Value};
use sqlx::PgConnection;
use url::Url;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    utils::{deserialize_url, empty_as_none, is_empty, RunnableKind},
    DB,
};
use windmill_store::resources::try_get_resource_from_db_as;

use windmill_api_auth::ApiAuthed;
use windmill_trigger_http::{
    http_trigger_auth::ApiKeyAuthentication, AuthenticationMethod, HttpMethod, RequestType,
};

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
const DEFAULT_SYNC_SSE_RESPONSE_KEY: &'static str = "SyncSseResponse";
const DEFAULT_PAYLOAD_PARAM_KEY: &'static str = "PayloadParam";

pub fn openapi_service() -> Router {
    Router::new()
        .route("/generate", post(generate_openapi_spec))
        .route("/download", post(download_spec))
}

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
    method: Method,
}

impl HttpRouteConfig {
    pub fn new(method: Method) -> Self {
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
    request_type: Option<RequestType>,
    summary: Option<String>,
    description: Option<String>,
    security_scheme: Option<SecurityScheme>,
}

impl FuturePath {
    pub fn new(
        route_path: String,
        kind: Kind,
        request_type: Option<RequestType>,
        summary: Option<String>,
        description: Option<String>,
        security_scheme: Option<SecurityScheme>,
    ) -> FuturePath {
        FuturePath { route_path, kind, request_type, summary, description, security_scheme }
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
            format!("/run_and_stream/{}", &normalized_path),
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

    let generate_response = |request_type: RequestType| {
        let responses = match request_type {
            RequestType::Async => serde_json::json!({
                "200": {
                    "$ref": format!("#/components/responses/{DEFAULT_ASYNC_RESPONSE_KEY}")
                }
            }),
            RequestType::Sync => serde_json::json!({
                "200": {
                    "$ref": format!("#/components/responses/{DEFAULT_SYNC_RESPONSE_KEY}")
                }
            }),
            RequestType::SyncSse => serde_json::json!({
                "200": {
                    "$ref": format!("#/components/responses/{DEFAULT_SYNC_SSE_RESPONSE_KEY}")
                }
            }),
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

    let mut webhooks = HashSet::new();

    for path in paths {
        if let Kind::Webhook(WebhookConfig { runnable_kind }) = &path.kind {
            if !webhooks.insert((path.route_path.clone(), runnable_kind.to_owned())) {
                continue;
            }
        }

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

            let request_type;

            let (methods, is_webhook) = match &path.kind {
                Kind::Webhook(_) => {
                    request_type = if route_path.starts_with("/run/") {
                        RequestType::Async
                    } else if route_path.starts_with("/run_and_stream/") {
                        RequestType::SyncSse
                    } else {
                        RequestType::Sync
                    };

                    let methods = if request_type == RequestType::Async {
                        vec![Method::POST]
                    } else {
                        vec![Method::GET, Method::POST]
                    };

                    (methods, true)
                }
                Kind::HttpRoute(HttpRouteConfig { method }) => {
                    if path_object.get(&method.to_string()).is_some() {
                        return Err(anyhow!(
                            "Found duplicate {} method, for route at path: {}",
                            method,
                            path.route_path
                        )
                        .into());
                    }
                    request_type = path.request_type.unwrap_or(RequestType::Sync);
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

                if method != Method::GET {
                    method_map.insert("requestBody", generate_default_request());
                } else if is_webhook {
                    method_map.insert(
                        "parameters",
                        Value::Array(vec![serde_json::json!({
                            "$ref": format!("#/components/parameters/{DEFAULT_PAYLOAD_PARAM_KEY}")
                        })]),
                    );
                }

                method_map.insert("responses", generate_response(request_type));

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
    api_keys: Vec<(String, Value)>,
}

fn generate_all_security_schemes(future_paths: &[FuturePath]) -> SecuritySchemeToAdd {
    let mut to_add = SecuritySchemeToAdd::default();

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
            to_add.api_keys.push((pascal_case_header, scheme));
        }
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

        let SecuritySchemeToAdd { basic_http, bearer_jwt, api_keys } =
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

        for (key, value) in api_keys {
            security_scheme.insert(key, value);
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
        DEFAULT_SYNC_SSE_RESPONSE_KEY: {
            "description": "Returns an SSE stream.",
            "content": {
                "text/event-stream": {
                    "schema": {
                        "type": "string",
                        "description": "Stream of SSE"
                    },
                }
            }
        }
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

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct HttpRouteFilter {
    folder_regex: String,
    path_regex: String,
    route_path_regex: String,
}

#[derive(Debug, Deserialize)]
struct WebhookFilter {
    user_or_folder_regex: String,
    user_or_folder_regex_value: String,
    path: String,
    runnable_kind: RunnableKind,
}

#[derive(Debug, Deserialize)]
struct GenerateOpenAPI {
    info: Option<Info>,
    url: Option<Url>,
    #[serde(default, deserialize_with = "empty_as_none")]
    http_route_filters: Option<Vec<HttpRouteFilter>>,
    #[serde(default, deserialize_with = "empty_as_none")]
    webhook_filters: Option<Vec<WebhookFilter>>,
    #[serde(default)]
    openapi_spec_format: Format,
}

async fn http_routes_to_future_paths(
    db: &DB,
    user_db: UserDB,
    authed: &ApiAuthed,
    pg_pool: &mut PgConnection,
    http_route_filters: Option<&[HttpRouteFilter]>,
    w_id: &str,
) -> Result<Vec<FuturePath>> {
    let mut http_routes = Vec::new();

    if let Some(http_route_filters) = http_route_filters {
        let path_regex = http_route_filters
            .iter()
            .map(|filter| {
                transform_to_minified_postgres_regex(&format!(
                    "f/{}/{}",
                    filter.folder_regex, filter.path_regex
                ))
            })
            .collect_vec();

        let route_path_regex = http_route_filters
            .iter()
            .map(|filter| transform_to_minified_postgres_regex(&filter.route_path_regex))
            .collect_vec();

        #[derive(Debug, Deserialize)]
        struct MinifiedHttpTrigger {
            route_path: String,
            http_method: HttpMethod,
            request_type: RequestType,
            workspaced_route: bool,
            summary: Option<String>,
            description: Option<String>,
            authentication_method: AuthenticationMethod,
            authentication_resource_path: Option<String>,
        }

        http_routes = sqlx::query_as!(
            MinifiedHttpTrigger,
            r#"
        SELECT
            route_path,
            http_method AS "http_method: _",
            request_type AS "request_type: _",
            workspaced_route,
            summary,
            description,
            authentication_method AS "authentication_method: _",
            authentication_resource_path
        FROM
            http_trigger
        WHERE
           path ~ ANY($1) AND
           route_path ~ ANY($2) AND
           workspace_id = $3
        "#,
            &path_regex,
            &route_path_regex,
            &w_id
        )
        .fetch_all(pg_pool)
        .await?;
    }

    let mut openapi_future_paths = Vec::with_capacity(http_routes.len());

    for http_route in http_routes {
        let auth_method = match http_route.authentication_method {
            AuthenticationMethod::BasicHttp => Some(SecurityScheme::BasicHttp),
            AuthenticationMethod::Windmill => Some(SecurityScheme::BearerJwt),
            AuthenticationMethod::ApiKey => {
                let resource_path = match http_route.authentication_resource_path {
                    Some(resource_path) => resource_path,
                    None => {
                        return Err(Error::BadRequest(
                            "Missing authentication resource path".to_string(),
                        ));
                    }
                };

                let api = try_get_resource_from_db_as::<ApiKeyAuthentication>(
                    authed,
                    Some(user_db.clone()),
                    db,
                    &resource_path,
                    w_id,
                )
                .await?;

                Some(SecurityScheme::ApiKey(api.api_key_header))
            }
            _ => None,
        };

        let route_path = if http_route.workspaced_route {
            format!("{}/{}", w_id, http_route.route_path.trim_start_matches('/'))
        } else {
            http_route.route_path.clone()
        };

        let method = match http_route.http_method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Delete => Method::DELETE,
        };

        let future_path = FuturePath::new(
            route_path,
            Kind::HttpRoute(HttpRouteConfig::new(method)),
            Some(http_route.request_type),
            http_route.summary,
            http_route.description,
            auth_method,
        );

        openapi_future_paths.push(future_path);
    }

    Ok(openapi_future_paths)
}

async fn webhook_to_future_paths(
    pg_pool: &mut PgConnection,
    webhook_filters: Option<&[WebhookFilter]>,
    w_id: &str,
) -> Result<Vec<FuturePath>> {
    let mut openapi_future_paths = Vec::new();
    if let Some(webhook_filters) = webhook_filters {
        let mut script_webhook_filter = Vec::new();
        let mut flow_webhook_filter = Vec::new();

        for webhook in webhook_filters {
            let full_regex = transform_to_minified_postgres_regex(&format!(
                "{}/{}/{}",
                &webhook.user_or_folder_regex, &webhook.user_or_folder_regex_value, &webhook.path
            ));

            match webhook.runnable_kind {
                RunnableKind::Script => {
                    script_webhook_filter.push(full_regex);
                }
                RunnableKind::Flow => {
                    flow_webhook_filter.push(full_regex);
                }
            }
        }

        #[derive(Debug, Deserialize, Clone, Hash)]
        struct MinifiedWebhook {
            path: String,
            description: Option<String>,
            summary: Option<String>,
        }

        let webhook_scripts = sqlx::query_as!(
            MinifiedWebhook,
            r#"SELECT 
                    path,
                    summary,
                    description
                FROM
                    script
                WHERE
                    path ~ ANY($1) AND
                    workspace_id = $2 AND
                    archived is FALSE
            "#,
            &script_webhook_filter,
            &w_id
        )
        .fetch_all(&mut *pg_pool)
        .await?;

        let webhook_flows = sqlx::query_as!(
            MinifiedWebhook,
            r#"SELECT 
                    path,
                    summary,
                    description
                FROM
                    flow
                WHERE
                    path ~ ANY($1) AND
                    workspace_id = $2 AND
                    archived is FALSE
            "#,
            &flow_webhook_filter,
            &w_id
        )
        .fetch_all(&mut *pg_pool)
        .await?;

        openapi_future_paths.reserve_exact(webhook_scripts.len() + webhook_flows.len());

        for webhook in webhook_scripts {
            openapi_future_paths.push(FuturePath::new(
                webhook.path,
                Kind::Webhook(WebhookConfig::new(RunnableKind::Script)),
                None,
                webhook.summary,
                webhook.description,
                Some(SecurityScheme::BearerJwt),
            ));
        }

        for webhook in webhook_flows {
            openapi_future_paths.push(FuturePath::new(
                webhook.path,
                Kind::Webhook(WebhookConfig::new(RunnableKind::Flow)),
                None,
                webhook.summary,
                webhook.description,
                Some(SecurityScheme::BearerJwt),
            ));
        }
    }

    Ok(openapi_future_paths)
}

async fn generate_openapi_future_path(
    db: &DB,
    user_db: UserDB,
    authed: &ApiAuthed,
    http_route_filters: Option<&[HttpRouteFilter]>,
    webhook_filters: Option<&[WebhookFilter]>,
    w_id: &str,
) -> Result<Vec<FuturePath>> {
    if http_route_filters.is_none() && webhook_filters.is_none() {
        return Err(Error::BadRequest(
            "Expected http route filter and/or webhook filters".to_string(),
        ));
    }

    let mut tx = user_db.clone().begin(authed).await?;

    let mut openapi_future_paths =
        http_routes_to_future_paths(db, user_db, authed, &mut tx, http_route_filters, w_id).await?;

    openapi_future_paths
        .append(&mut webhook_to_future_paths(&mut tx, webhook_filters, w_id).await?);

    tx.commit().await?;

    if openapi_future_paths.is_empty() {
        return Err(Error::NotFound(
            "No match for the current filter".to_string(),
        ));
    }

    Ok(openapi_future_paths)
}

async fn generate_openapi_spec(
    Extension(authed): Extension<ApiAuthed>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(generate_openapi): Json<GenerateOpenAPI>,
) -> Result<String> {
    let openapi_future_paths = generate_openapi_future_path(
        &db,
        user_db,
        &authed,
        generate_openapi.http_route_filters.as_deref(),
        generate_openapi.webhook_filters.as_deref(),
        &w_id,
    )
    .await?;

    let openapi_document = generate_openapi_document(
        generate_openapi.info.as_ref(),
        generate_openapi.url.as_ref(),
        openapi_future_paths,
        generate_openapi.openapi_spec_format,
    );

    openapi_document
}

async fn download_spec(
    Extension(authed): Extension<ApiAuthed>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(generate_openapi): Json<GenerateOpenAPI>,
) -> Result<Response> {
    let openapi_future_paths = generate_openapi_future_path(
        &db,
        user_db,
        &authed,
        generate_openapi.http_route_filters.as_deref(),
        generate_openapi.webhook_filters.as_deref(),
        &w_id,
    )
    .await?;

    let openapi_document = generate_openapi_document(
        generate_openapi.info.as_ref(),
        generate_openapi.url.as_ref(),
        openapi_future_paths,
        generate_openapi.openapi_spec_format,
    )?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        )
        .body(Body::from(openapi_document))
        .unwrap();

    Ok(response)
}

use std::collections::HashMap;

use axum::{
    extract::{FromRequest, Multipart, Query, Request},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::JsonRawValue;
use windmill_common::{
    error::Error,
    triggers::{RunnableFormat, RunnableFormatVersion, TriggerKind},
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;

use crate::{
    db::ApiAuthed,
    triggers::trigger_helpers::{get_runnable_format, RunnableId},
};

#[derive(Debug)]
pub enum RawBody {
    Json(String),
    CEJson(String),
    Text(String),
    Xml(String),
    UrlEncoded(Bytes),
    Multipart(Multipart),
    Empty,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Body {
    HashMap(HashMap<String, Box<RawValue>>),
    NoHashMap(Box<RawValue>),
}

#[derive(Debug, Clone, Default)]
pub struct WebhookArgsMetadata {
    pub raw_string: Option<String>,
    pub headers: HeaderMap,
    pub query: Option<String>,
    pub method: http::Method,
    pub query_wrap_body: bool,
    pub query_use_raw: bool,
    pub query_include_header: Option<String>,
    pub query_include_query: Option<String>,
}

pub struct RawWebhookArgs {
    pub body: RawBody,
    pub metadata: WebhookArgsMetadata,
}

#[derive(Debug, Clone)]
pub struct WebhookArgs {
    pub body: Body,
    pub metadata: WebhookArgsMetadata,
}

// capture
//

impl RawWebhookArgs {
    #[cfg(not(feature = "parquet"))]
    pub async fn process_multipart(
        _multipart: Multipart,
        _authed: &ApiAuthed,
        _db: &DB,
        _w_id: &str,
    ) -> Result<HashMap<String, Box<RawValue>>, Error> {
        return Err(Error::BadRequest(format!(
            "multipart/form-data requires the parquet feature"
        )));
    }

    #[cfg(feature = "parquet")]
    async fn process_multipart(
        mut multipart: Multipart,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
    ) -> Result<HashMap<String, Box<RawValue>>, Error> {
        use crate::job_helpers_oss::{
            get_random_file_name, get_workspace_s3_resource, upload_file_internal,
        };
        use futures::TryStreamExt;
        use windmill_object_store::object_store_reexports::{Attribute, Attributes};
        use windmill_object_store::build_object_store_client;

        let (_, s3_resource) = get_workspace_s3_resource(authed, db, None, w_id, None).await?;

        if let Some(s3_resource) = s3_resource {
            let s3_client = build_object_store_client(&s3_resource).await?;

            let mut body = HashMap::new();
            let mut files = HashMap::new();

            while let Some(field) = multipart.next_field().await.map_err(|e| {
                Error::BadRequest(format!("Error reading multipart field: {}", e.body_text()))
            })? {
                if let Some(name) = field.name().map(|x| x.to_string()) {
                    if let Some(content_type) = field.content_type() {
                        let ext = field
                            .file_name()
                            .map(|x| x.split('.').last())
                            .flatten()
                            .map(|x| x.to_string());

                        let file_key = get_random_file_name(ext);

                        let options = Attributes::from_iter(vec![
                            (Attribute::ContentType, content_type.to_string()),
                            (
                                Attribute::ContentDisposition,
                                if let Some(filename) = field.file_name() {
                                    format!("inline; filename=\"{}\"", filename)
                                } else {
                                    "inline".to_string()
                                },
                            ),
                        ])
                        .into();

                        let bytes_stream = field
                            .into_stream()
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));

                        upload_file_internal(s3_client.clone(), &file_key, bytes_stream, options)
                            .await?;

                        files.entry(name).or_insert(vec![]).push(serde_json::json!({
                            "s3": &file_key
                        }));
                    } else {
                        body.insert(name, to_raw_value(&field.text().await.unwrap_or_default()));
                    }
                }
            }

            for (k, v) in files {
                body.insert(k, to_raw_value(&v));
            }

            Ok(body)
        } else {
            Err(Error::BadRequest(format!(
                "You need to connect your workspace to an S3 bucket to use multipart/form-data"
            )))
        }
    }

    pub async fn process_args(
        self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
        force_use_raw: Option<bool>,
    ) -> Result<WebhookArgs, Error> {
        let use_raw = force_use_raw.unwrap_or(self.metadata.query_use_raw);

        match self.body {
            RawBody::Multipart(multipart) => {
                let body = Self::process_multipart(multipart, authed, db, w_id).await?;
                Ok(WebhookArgs { body: Body::HashMap(body), metadata: self.metadata })
            }
            RawBody::Empty => {
                let mut metadata = self.metadata;
                if use_raw {
                    metadata.raw_string = Some("".to_string());
                }
                Ok(WebhookArgs { body: Body::HashMap(HashMap::new()), metadata })
            }
            RawBody::Text(s) | RawBody::Xml(s) => Ok(WebhookArgs {
                body: Body::HashMap(HashMap::new()),
                metadata: WebhookArgsMetadata { raw_string: Some(s), ..self.metadata },
            }),
            RawBody::UrlEncoded(bytes) => {
                let mut metadata = self.metadata;
                if use_raw {
                    let raw_string = String::from_utf8(bytes.to_vec())
                        .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)))?;
                    metadata.raw_string = Some(raw_string);
                }
                let payload: HashMap<String, Option<String>> = serde_urlencoded::from_bytes(&bytes)
                    .map_err(|e| Error::BadRequest(format!("invalid urlencoded data: {}", e)))?;
                let payload = payload
                    .into_iter()
                    .map(|(k, v)| (k, to_raw_value(&v)))
                    .collect::<HashMap<_, _>>();

                Ok(WebhookArgs { body: Body::HashMap(payload), metadata })
            }
            RawBody::Json(s) => WebhookArgs::from_json(self.metadata, use_raw, s).await,
            RawBody::CEJson(s) => WebhookArgs::from_ce_json(self.metadata, use_raw, s).await,
        }
    }

    pub async fn to_main_args(
        self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
    ) -> Result<PushArgsOwned, Error> {
        let args = self.process_args(authed, db, w_id, None).await?;
        args.to_main_args()
    }

    pub async fn to_args_from_runnable(
        self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
        runnable_id: RunnableId,
        skip_preprocessor: Option<bool>,
    ) -> Result<PushArgsOwned, Error> {
        let args = self.process_args(authed, db, w_id, None).await?;
        args.to_args_from_runnable(db, w_id, runnable_id, skip_preprocessor)
            .await
    }
}

#[derive(Serialize)]
struct WebhookPreprocessorEvent {
    kind: String,
    body: Box<RawValue>,
    raw_string: Option<String>,
    headers: HashMap<String, Box<RawValue>>,
    query: HashMap<String, Box<RawValue>>,
}

impl WebhookArgs {
    pub fn to_main_args(self) -> Result<PushArgsOwned, Error> {
        self.to_args_from_format(RunnableFormat {
            has_preprocessor: false,
            version: RunnableFormatVersion::V2,
        })
    }

    pub async fn to_args_from_runnable(
        self,
        db: &DB,
        w_id: &str,
        runnable_id: RunnableId,
        skip_preprocessor: Option<bool>,
    ) -> Result<PushArgsOwned, Error> {
        if skip_preprocessor.unwrap_or(false) {
            self.to_main_args()
        } else {
            let runnable_format =
                get_runnable_format(runnable_id, w_id, db, &TriggerKind::Webhook).await?;

            self.to_args_from_format(runnable_format)
        }
    }

    pub fn to_args_from_format(
        self,
        runnable_format: RunnableFormat,
    ) -> Result<PushArgsOwned, Error> {
        let headers = build_headers(
            &self.metadata.headers,
            self.metadata.query_include_header,
            runnable_format.has_preprocessor,
        );

        let query = build_query(
            self.metadata.query.as_deref(),
            self.metadata.query_include_query,
            runnable_format.has_preprocessor,
        );

        match runnable_format {
            RunnableFormat { has_preprocessor: true, version: RunnableFormatVersion::V2 } => {
                let mut args = HashMap::new();

                args.insert(
                    "event".to_string(),
                    to_raw_value(&WebhookPreprocessorEvent {
                        kind: "webhook".to_string(),
                        body: to_raw_value(&self.body),
                        raw_string: self.metadata.raw_string,
                        headers,
                        query,
                    }),
                );

                Ok(PushArgsOwned { args, extra: None })
            }
            RunnableFormat { has_preprocessor, .. } => {
                let mut extra = HashMap::new();

                let WebhookArgsMetadata { query_wrap_body, raw_string, .. } = self.metadata;

                for (k, v) in headers {
                    extra.insert(k, v);
                }

                for (k, v) in query {
                    extra.insert(k, v);
                }

                if let Some(raw_string) = raw_string {
                    extra.insert("raw_string".to_string(), to_raw_value(&raw_string));
                }

                if has_preprocessor {
                    // if has preprocessor, it has to be v1
                    extra.insert(
                        "wm_trigger".to_string(),
                        to_raw_value(&serde_json::json!({
                            "kind": "webhook",
                        })),
                    );
                }

                let extra = if extra.is_empty() { None } else { Some(extra) };

                match self.body {
                    Body::HashMap(mut body) => {
                        if query_wrap_body {
                            body = HashMap::from([("body".to_string(), to_raw_value(&body))]);
                        }
                        Ok(PushArgsOwned { args: body, extra })
                    }
                    Body::NoHashMap(args) => {
                        let mut hm = HashMap::new();
                        hm.insert("body".to_string(), args);
                        Ok(PushArgsOwned { args: hm, extra })
                    }
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct RequestQuery {
    pub raw: Option<bool>,
    pub wrap_body: Option<bool>,
    pub include_header: Option<String>,
    pub include_query: Option<String>,
}

async fn req_to_string<S: Send + Sync>(
    req: Request<axum::body::Body>,
    _state: &S,
) -> Result<String, Response> {
    let bytes = Bytes::from_request(req, _state)
        .await
        .map_err(IntoResponse::into_response)?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)).into_response())
}

pub async fn try_from_request_body<S>(
    request: Request,
    _state: &S,
    is_http_trigger: bool,
) -> Result<RawWebhookArgs, Response>
where
    S: Send + Sync,
{
    let (content_type, metadata) = {
        let headers_map = request.headers();
        let content_type_header = headers_map.get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());
        let uri = request.uri();
        let request_query = Query::<RequestQuery>::try_from_uri(uri).unwrap().0;

        let query = uri.query().map(|s| s.to_owned());
        let raw = !is_http_trigger && request_query.raw.unwrap_or(false);
        let wrap_body = !is_http_trigger && request_query.wrap_body.unwrap_or(false);
        (
            content_type,
            WebhookArgsMetadata {
                headers: headers_map.clone(),
                query,
                method: request.method().clone(),
                raw_string: None,
                query_wrap_body: wrap_body,
                query_use_raw: raw,
                query_include_header: request_query.include_header,
                query_include_query: request_query.include_query,
            },
        )
    };

    let no_content_type = content_type.is_none();
    if no_content_type || content_type.unwrap().starts_with("application/json") {
        let bytes = Bytes::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;
        if no_content_type && bytes.is_empty() {
            Ok(RawWebhookArgs { body: RawBody::Empty, metadata })
        } else {
            let str = String::from_utf8(bytes.to_vec())
                .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)).into_response())?;
            Ok(RawWebhookArgs { body: RawBody::Json(str), metadata })
        }
    } else if content_type
        .unwrap()
        .starts_with("application/cloudevents+json")
    {
        let str = req_to_string(request, _state).await?;

        Ok(RawWebhookArgs { body: RawBody::CEJson(str), metadata })
    } else if content_type
        .unwrap()
        .starts_with("application/cloudevents-batch+json")
    {
        Err(
            Error::BadRequest(format!("Cloud events batching is not supported yet"))
                .into_response(),
        )
    } else if content_type.unwrap().starts_with("text/plain") {
        let str = req_to_string(request, _state).await?;
        Ok(RawWebhookArgs { body: RawBody::Text(str), metadata })
    } else if content_type
        .unwrap()
        .starts_with("application/x-www-form-urlencoded")
    {
        let bytes = Bytes::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        Ok(RawWebhookArgs { body: RawBody::UrlEncoded(bytes), metadata })
    } else if content_type.unwrap().starts_with("application/xml")
        || content_type.unwrap().starts_with("text/xml")
    {
        let str = req_to_string(request, _state).await?;
        Ok(RawWebhookArgs { body: RawBody::Xml(str), metadata })
    } else if content_type.unwrap().starts_with("multipart/form-data") {
        let multipart = Multipart::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        Ok(RawWebhookArgs { body: RawBody::Multipart(multipart), metadata })
    } else {
        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for RawWebhookArgs
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let args = try_from_request_body(request, _state, false).await?;

        Ok(args)
    }
}

lazy_static::lazy_static! {
    static ref INCLUDE_HEADERS: Vec<String> = std::env::var("INCLUDE_HEADERS")
        .ok().map(|x| x
        .split(',')
        .map(|s| s.to_string())
        .collect()).unwrap_or_default();
}

pub fn build_headers(
    headers: &HeaderMap,
    include_header: Option<String>,
    include_all_headers: bool,
) -> HashMap<String, Box<RawValue>> {
    let mut selected_headers = HashMap::new();

    if include_all_headers {
        for (k, v) in headers.iter() {
            selected_headers.insert(
                k.to_string(),
                to_raw_value(&v.to_str().unwrap_or("").to_string()),
            );
        }
    } else {
        let whitelist = include_header
            .map(|s| s.split(",").map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap_or_default();
        whitelist
            .iter()
            .chain(INCLUDE_HEADERS.iter())
            .for_each(|h| {
                if let Some(v) = headers.get(h) {
                    selected_headers.insert(
                        h.to_string().to_lowercase().replace('-', "_"),
                        to_raw_value(&v.to_str().unwrap_or("").to_string()),
                    );
                }
            });
    }
    selected_headers
}

pub fn build_query(
    query: Option<&str>,
    include_query: Option<String>,
    include_all_query: bool,
) -> HashMap<String, Box<RawValue>> {
    let Some(query) = query else {
        return HashMap::new();
    };

    if include_all_query {
        let queries =
            serde_urlencoded::from_str::<HashMap<String, String>>(&query).unwrap_or_default();
        queries
            .into_iter()
            .map(|(k, v)| (k, to_raw_value(&v)))
            .collect()
    } else {
        let parse_query_args = include_query
            .map(|s| s.split(",").map(|p| p.to_string()).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut args = HashMap::new();
        if !parse_query_args.is_empty() {
            let queries =
                serde_urlencoded::from_str::<HashMap<String, String>>(&query).unwrap_or_default();
            parse_query_args.iter().for_each(|h| {
                if let Some(v) = queries.get(h) {
                    args.insert(h.to_string(), to_raw_value(v));
                }
            });
        }
        args
    }
}

fn restructure_cloudevents_metadata(
    mut p: HashMap<String, Box<RawValue>>,
) -> Result<HashMap<String, Box<RawValue>>, Error> {
    let data = p
        .remove("data")
        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
    let str = data.to_string();

    let wrap_body = str.len() > 0 && str.chars().next().unwrap() != '{';

    if wrap_body {
        let args = serde_json::from_str::<Option<Box<RawValue>>>(&str)
            .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?
            .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
        let mut hm = HashMap::new();
        hm.insert("body".to_string(), args);
        hm.insert("WEBHOOK__METADATA__".to_string(), to_raw_value(&p));
        Ok(hm)
    } else {
        let mut hm = serde_json::from_str::<Option<HashMap<String, Box<JsonRawValue>>>>(&str)
            .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?
            .unwrap_or_else(HashMap::new);
        hm.insert("WEBHOOK__METADATA__".to_string(), to_raw_value(&p));
        Ok(hm)
    }
}

impl WebhookArgs {
    async fn from_json(
        mut metadata: WebhookArgsMetadata,
        use_raw: bool,
        str: String,
    ) -> Result<Self, Error> {
        if use_raw {
            metadata.raw_string = Some(str.clone());
        }

        let no_hashmap = str.len() > 0 && str.chars().next().unwrap() != '{';

        if no_hashmap {
            let args = serde_json::from_str::<Option<Box<RawValue>>>(&str)
                .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?
                .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));

            Ok(Self { body: Body::NoHashMap(args), metadata })
        } else {
            let hm = serde_json::from_str::<Option<HashMap<String, Box<JsonRawValue>>>>(&str)
                .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?
                .unwrap_or_else(HashMap::new);
            Ok(Self { body: Body::HashMap(hm), metadata })
        }
    }

    async fn from_ce_json(
        mut metadata: WebhookArgsMetadata,
        use_raw: bool,
        str: String,
    ) -> Result<Self, Error> {
        if use_raw {
            metadata.raw_string = Some(str.clone());
        }

        let hm = serde_json::from_str::<HashMap<String, Box<RawValue>>>(&str)
            .map_err(|e| Error::BadRequest(format!("invalid cloudevents+json: {}", e)))?;
        let hm = restructure_cloudevents_metadata(hm)?;
        Ok(Self { body: Body::HashMap(hm), metadata })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_cloudevents_json_payload() {
        let r1 = r#"
        {
            "specversion" : "1.0",
            "type" : "com.example.someevent",
            "source" : "/mycontext",
            "subject": null,
            "id" : "C234-1234-1234",
            "time" : "2018-04-05T17:31:00Z",
            "comexampleextension1" : "value",
            "comexampleothervalue" : 5,
            "datacontenttype" : "application/json",
            "data" : {
                "appinfoA" : "abc",
                "appinfoB" : 123,
                "appinfoC" : true
            }
        }
        "#;
        let r2 = r#"
        {
            "specversion" : "1.0",
            "type" : "com.example.someevent",
            "source" : "/mycontext",
            "subject": null,
            "id" : "C234-1234-1234",
            "time" : "2018-04-05T17:31:00Z",
            "comexampleextension1" : "value",
            "comexampleothervalue" : 5,
            "datacontenttype" : "application/json",
            "data" : 1.5
        }
        "#;
        let metadata = WebhookArgsMetadata::default();

        let a1 = WebhookArgs::from_ce_json(metadata.clone(), false, r1.to_string())
            .await
            .expect("Failed to parse the cloudevent");
        let a2 = WebhookArgs::from_ce_json(metadata.clone(), false, r2.to_string())
            .await
            .expect("Failed to parse the cloudevent");

        match a1.body {
            Body::HashMap(body) => {
                body.get("WEBHOOK__METADATA__").expect(
                    "CloudEvents should generate a neighboring `webhook-metadata` field in PushArgs",
                );
            }
            _ => panic!("Expected a HashMap"),
        }

        match a2.body {
            Body::HashMap(body) => {
                assert_eq!(
                    body
                        .get("body")
                        .expect("Cloud events with a data field with no wrapping curly brackets should be inside of a `body` field in PushArgs")
                        .to_string(),
                    "1.5"
                );
            }
            _ => panic!("Expected a HashMap"),
        }
    }
}

use std::collections::HashMap;

#[cfg(feature = "parquet")]
use crate::job_helpers_ee::get_workspace_s3_resource;
use axum::{
    extract::{FromRequest, FromRequestParts, Multipart, Query, Request},
    http::{HeaderMap, Uri},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, request::Parts, StatusCode};
#[cfg(feature = "parquet")]
use object_store::{Attribute, Attributes};
use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::types::JsonRawValue;
#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::build_object_store_client;
use windmill_common::{error::Error, worker::to_raw_value, DB};
use windmill_queue::PushArgsOwned;

use crate::db::ApiAuthed;
#[cfg(feature = "parquet")]
use crate::job_helpers_ee::{get_random_file_name, upload_file_internal};

#[derive(Debug, Default)]
pub struct WebhookArgs {
    pub args: PushArgsOwned,
    pub multipart: Option<Multipart>,
    pub wrap_body: Option<bool>,
}

impl WebhookArgs {
    #[cfg(not(feature = "parquet"))]
    pub async fn to_push_args_owned(
        self,
        _authed: &ApiAuthed,
        _db: &DB,
        _w_id: &str,
    ) -> Result<PushArgsOwned, Error> {
        if self.multipart.is_some() {
            return Err(Error::BadRequest(format!(
                "multipart/form-data requires the parquet feature"
            )));
        }

        Ok(self.args)
    }

    #[cfg(feature = "parquet")]
    pub async fn to_push_args_owned(
        mut self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
    ) -> Result<PushArgsOwned, Error> {
        use futures::TryStreamExt;

        if let Some(mut multipart) = self.multipart {
            {
                let (_, s3_resource) =
                    get_workspace_s3_resource(authed, db, None, "", w_id, None).await?;

                if let Some(s3_resource) = s3_resource {
                    let s3_client = build_object_store_client(&s3_resource).await?;

                    let mut body = HashMap::new();
                    let mut files = HashMap::new();

                    while let Some(field) = multipart.next_field().await.map_err(|e| {
                        Error::BadRequest(format!(
                            "Error reading multipart field: {}",
                            e.body_text()
                        ))
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

                                let bytes_stream = field.into_stream().map_err(|err| {
                                    std::io::Error::new(std::io::ErrorKind::Other, err)
                                });

                                upload_file_internal(
                                    s3_client.clone(),
                                    &file_key,
                                    bytes_stream,
                                    options,
                                )
                                .await?;

                                files.entry(name).or_insert(vec![]).push(serde_json::json!({
                                    "s3": &file_key
                                }));
                            } else {
                                body.insert(
                                    name,
                                    to_raw_value(&field.text().await.unwrap_or_default()),
                                );
                            }
                        }
                    }

                    for (k, v) in files {
                        body.insert(k, to_raw_value(&v));
                    }

                    if self.wrap_body.unwrap_or(false) {
                        self.args
                            .args
                            .insert("body".to_string(), to_raw_value(&body));
                    } else {
                        self.args.args.extend(body);
                    }

                    return Ok(self.args);
                }
            }

            return Err(Error::BadRequest(format!(
                "You need to connect your workspace to an S3 bucket to use multipart/form-data"
            )));
        }

        Ok(self.args)
    }
}

#[derive(Deserialize)]
pub struct RequestQuery {
    pub raw: Option<bool>,
    pub wrap_body: Option<bool>,
    pub include_header: Option<String>,
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
    use_raw: Option<bool>,
    wrap_body: Option<bool>,
) -> Result<WebhookArgs, Response>
where
    S: Send + Sync,
{
    let (content_type, mut extra, use_raw, wrap_body) = {
        let headers_map = request.headers();
        let content_type_header = headers_map.get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());
        let uri = request.uri();
        let query = Query::<RequestQuery>::try_from_uri(uri).unwrap().0;
        let mut extra = build_extra(&headers_map, query.include_header);
        let query_decode = DecodeQueries::from_uri(uri);
        if let Some(DecodeQueries(queries)) = query_decode {
            extra.extend(queries);
        }
        let raw = query.raw.unwrap_or(use_raw.unwrap_or(false));
        let wrap_body = query.wrap_body.unwrap_or(wrap_body.unwrap_or(false));
        (content_type, extra, raw, wrap_body)
    };

    let no_content_type = content_type.is_none();
    if no_content_type || content_type.unwrap().starts_with("application/json") {
        let bytes = Bytes::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;
        if no_content_type && bytes.is_empty() {
            if use_raw {
                extra.insert("raw_string".to_string(), to_raw_value(&"".to_string()));
            }
            let mut args = HashMap::new();
            if wrap_body {
                args.insert("body".to_string(), to_raw_value(&serde_json::json!({})));
            }
            return Ok(WebhookArgs {
                args: PushArgsOwned { extra: Some(extra), args: args },
                ..Default::default()
            });
        }
        let str = String::from_utf8(bytes.to_vec())
            .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)).into_response())?;

        PushArgsOwned::from_json(extra, use_raw, wrap_body, str)
            .await
            .map(|args| WebhookArgs { args, ..Default::default() })
    } else if content_type
        .unwrap()
        .starts_with("application/cloudevents+json")
    {
        let str = req_to_string(request, _state).await?;

        PushArgsOwned::from_ce_json(extra, use_raw, str)
            .await
            .map(|args| WebhookArgs { args, ..Default::default() })
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
        extra.insert("raw_string".to_string(), to_raw_value(&str));
        Ok(WebhookArgs {
            args: PushArgsOwned { extra: Some(extra), args: HashMap::new() },
            ..Default::default()
        })
    } else if content_type
        .unwrap()
        .starts_with("application/x-www-form-urlencoded")
    {
        let bytes = Bytes::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        if use_raw {
            let raw_string = String::from_utf8(bytes.to_vec())
                .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)).into_response())?;
            extra.insert("raw_string".to_string(), to_raw_value(&raw_string));
        }

        let payload: HashMap<String, Option<String>> = serde_urlencoded::from_bytes(&bytes)
            .map_err(|e| {
                Error::BadRequest(format!("invalid urlencoded data: {}", e)).into_response()
            })?;
        let payload = payload
            .into_iter()
            .map(|(k, v)| (k, to_raw_value(&v)))
            .collect::<HashMap<_, _>>();

        return Ok(WebhookArgs {
            args: PushArgsOwned { extra: Some(extra), args: payload },
            ..Default::default()
        });
    } else if content_type.unwrap().starts_with("application/xml")
        || content_type.unwrap().starts_with("text/xml")
    {
        let str = req_to_string(request, _state).await?;
        extra.insert("raw_string".to_string(), to_raw_value(&str));
        Ok(WebhookArgs {
            args: PushArgsOwned { extra: Some(extra), args: HashMap::new() },
            ..Default::default()
        })
    } else if content_type.unwrap().starts_with("multipart/form-data") {
        let multipart = Multipart::from_request(request, _state)
            .await
            .map_err(IntoResponse::into_response)?;

        Ok(WebhookArgs {
            args: PushArgsOwned { extra: Some(extra), args: HashMap::new() },
            multipart: Some(multipart),
            wrap_body: Some(wrap_body),
        })
    } else {
        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for WebhookArgs
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let args = try_from_request_body(request, _state, None, None).await?;

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

pub fn build_extra(
    headers: &HeaderMap,
    include_header: Option<String>,
) -> HashMap<String, Box<RawValue>> {
    let mut args = HashMap::new();
    let whitelist = include_header
        .map(|s| s.split(",").map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    whitelist
        .iter()
        .chain(INCLUDE_HEADERS.iter())
        .for_each(|h| {
            if let Some(v) = headers.get(h) {
                args.insert(
                    h.to_string().to_lowercase().replace('-', "_"),
                    to_raw_value(&v.to_str().unwrap().to_string()),
                );
            }
        });
    args
}

#[derive(Deserialize)]
pub struct IncludeQuery {
    pub include_query: Option<String>,
}

pub struct DecodeQueries(pub HashMap<String, Box<RawValue>>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for DecodeQueries
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(DecodeQueries::from_uri(&parts.uri).unwrap_or_else(|| DecodeQueries(HashMap::new())))
    }
}

impl DecodeQueries {
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        let query = uri.query();
        if query.is_none() {
            return None;
        }
        let query = query.unwrap();
        let include_query = serde_urlencoded::from_str::<IncludeQuery>(query)
            .map(|x| x.include_query)
            .ok()
            .flatten()
            .unwrap_or_default();
        let parse_query_args = include_query
            .split(",")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let mut args = HashMap::new();
        if !parse_query_args.is_empty() {
            let queries =
                serde_urlencoded::from_str::<HashMap<String, String>>(query).unwrap_or_default();
            parse_query_args.iter().for_each(|h| {
                if let Some(v) = queries.get(h) {
                    args.insert(h.to_string(), to_raw_value(v));
                }
            });
        }
        Some(DecodeQueries(args))
    }
}

// impl<'c> PushArgs<'c> {
//     pub fn insert<K: Into<String>, V: Into<Box<RawValue>>>(&mut self, k: K, v: V) {
//         self.extra.insert(k.into(), v.into());
//     }
// }

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

trait PushArgsOwnedExt: Sized {
    async fn from_json(
        extra: HashMap<String, Box<RawValue>>,
        use_raw: bool,
        force_wrap_body: bool,
        str: String,
    ) -> Result<Self, Response>;

    async fn from_ce_json(
        extra: HashMap<String, Box<RawValue>>,
        use_raw: bool,
        str: String,
    ) -> Result<Self, Response>;
}

impl PushArgsOwnedExt for PushArgsOwned {
    async fn from_json(
        mut extra: HashMap<String, Box<RawValue>>,
        use_raw: bool,
        force_wrap_body: bool,
        str: String,
    ) -> Result<Self, Response> {
        if use_raw {
            extra.insert("raw_string".to_string(), to_raw_value(&str));
        }

        let wrap_body = force_wrap_body || str.len() > 0 && str.chars().next().unwrap() != '{';

        if wrap_body {
            let args = serde_json::from_str::<Option<Box<RawValue>>>(&str)
                .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)).into_response())?
                .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
            let mut hm = HashMap::new();
            hm.insert("body".to_string(), args);
            Ok(PushArgsOwned { extra: Some(extra), args: hm })
        } else {
            let hm = serde_json::from_str::<Option<HashMap<String, Box<JsonRawValue>>>>(&str)
                .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)).into_response())?
                .unwrap_or_else(HashMap::new);
            Ok(PushArgsOwned { extra: Some(extra), args: hm })
        }
    }

    async fn from_ce_json(
        mut extra: HashMap<String, Box<RawValue>>,
        use_raw: bool,
        str: String,
    ) -> Result<Self, Response> {
        if use_raw {
            extra.insert("raw_string".to_string(), to_raw_value(&str));
        }

        let hm = serde_json::from_str::<HashMap<String, Box<RawValue>>>(&str).map_err(|e| {
            Error::BadRequest(format!("invalid cloudevents+json: {}", e)).into_response()
        })?;
        let hm = restructure_cloudevents_metadata(hm).map_err(|e| e.into_response())?;
        Ok(PushArgsOwned { extra: Some(extra), args: hm })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
        let extra = HashMap::new();

        let a1 = PushArgsOwned::from_ce_json(extra.clone(), false, r1.to_string())
            .await
            .expect("Failed to parse the cloudevent");
        let a2 = PushArgsOwned::from_ce_json(extra.clone(), false, r2.to_string())
            .await
            .expect("Failed to parse the cloudevent");

        a1.args.get("WEBHOOK__METADATA__").expect(
            "CloudEvents should generate a neighboring `webhook-metadata` field in PushArgs",
        );
        assert_eq!(
            a2.args
                .get("body")
                .expect("Cloud events with a data field with no wrapping curly brackets should be inside of a `body` field in PushArgs")
                .to_string(),
            "1.5"
        );
    }
}

use std::collections::HashMap;

use axum::{
    extract::{FromRequest, Request},
    response::Response,
};
use serde::Serialize;
use serde_json::value::RawValue;
use windmill_common::{
    error::Error,
    triggers::{RunnableFormat, RunnableFormatVersion},
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;

use super::HttpMethod;
use crate::{
    args::{
        build_headers, build_query, try_from_request_body, Body, RawWebhookArgs, WebhookArgs,
        WebhookArgsMetadata,
    },
    db::ApiAuthed,
};

pub struct RawHttpTriggerArgs(pub RawWebhookArgs);

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for RawHttpTriggerArgs
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let args = try_from_request_body(request, _state, true).await?;

        Ok(Self(args))
    }
}

#[derive(Debug, Clone)]
pub struct HttpTriggerArgs(pub WebhookArgs);

impl RawHttpTriggerArgs {
    pub async fn process_args(
        self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
        use_raw: bool,
    ) -> Result<HttpTriggerArgs, Error> {
        if self.0.metadata.query_use_raw || self.0.metadata.query_wrap_body {
            return Err(Error::BadRequest(
                "Specifying use raw or wrap body with query args is not supported anymore on http routes, please set it in the trigger config".to_string(),
            )
            .into());
        }

        let args = self.0.process_args(authed, db, w_id, Some(use_raw)).await?;

        Ok(HttpTriggerArgs(args))
    }
}

#[derive(Serialize)]
struct HttpTriggerPreprocessorEvent<'a> {
    kind: String,
    route: &'a str,
    path: &'a str,
    body: Box<RawValue>,
    raw_string: Option<String>,
    params: &'a HashMap<String, String>,
    headers: HashMap<String, Box<RawValue>>,
    query: HashMap<String, Box<RawValue>>,
    method: HttpMethod,
}

#[derive(Serialize)]
struct HttpTriggerWmTrigger<'a> {
    route: &'a str,
    path: &'a str,
    params: &'a HashMap<String, String>,
    query: HashMap<String, Box<RawValue>>,
    headers: HashMap<String, Box<RawValue>>,
    method: HttpMethod,
}

impl HttpTriggerArgs {
    pub fn to_main_args(self, wrap_body: bool) -> Result<PushArgsOwned, Error> {
        let mut extra = HashMap::new();

        let WebhookArgsMetadata { raw_string, .. } = self.0.metadata;

        if let Some(raw_string) = raw_string {
            extra.insert("raw_string".to_string(), to_raw_value(&raw_string));
        }

        let extra = if extra.is_empty() { None } else { Some(extra) };

        match self.0.body {
            Body::HashMap(mut body) => {
                if wrap_body {
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

    pub fn to_args_from_format(
        self,
        route_path: &str,
        called_path: &str,
        params: &HashMap<String, String>,
        format: RunnableFormat,
        wrap_body: bool,
    ) -> Result<PushArgsOwned, Error> {
        let headers = build_headers(&self.0.metadata.headers, None, true);
        let query = build_query(self.0.metadata.query.as_deref(), None, true);
        match format {
            RunnableFormat { has_preprocessor: true, version: RunnableFormatVersion::V2 } => {
                // we don't care about wrap_body in v2
                self.to_v2_preprocessor_args(route_path, called_path, params, headers, query)
            }
            RunnableFormat { has_preprocessor: true, version: RunnableFormatVersion::V1 } => self
                .to_v1_preprocessor_args(
                    route_path,
                    called_path,
                    params,
                    wrap_body,
                    headers,
                    query,
                ),
            RunnableFormat { has_preprocessor: false, .. } => self.to_main_args(wrap_body),
        }
    }

    fn to_v1_preprocessor_args(
        self,
        route_path: &str,
        called_path: &str,
        params: &HashMap<String, String>,
        wrap_body: bool,
        headers: HashMap<String, Box<RawValue>>,
        query: HashMap<String, Box<RawValue>>,
    ) -> Result<PushArgsOwned, Error> {
        let mut extra = HashMap::new();
        let mut wm_trigger = HashMap::new();
        wm_trigger.insert("kind".to_string(), to_raw_value(&"http".to_string()));
        wm_trigger.insert(
            "http".to_string(),
            to_raw_value(&HttpTriggerWmTrigger {
                route: route_path,
                path: called_path,
                method: (&self.0.metadata.method).try_into()?,
                params,
                query,
                headers,
            }),
        );
        extra.insert("wm_trigger".to_string(), to_raw_value(&wm_trigger));

        let mut args = self.to_main_args(wrap_body)?;

        args.extra.get_or_insert_default().extend(extra);

        Ok(args)
    }

    pub fn to_v2_preprocessor_args(
        self,
        route_path: &str,
        called_path: &str,
        params: &HashMap<String, String>,
        headers: HashMap<String, Box<RawValue>>,
        query: HashMap<String, Box<RawValue>>,
    ) -> Result<PushArgsOwned, Error> {
        let mut args = HashMap::new();
        args.insert(
            "event".to_string(),
            to_raw_value(&HttpTriggerPreprocessorEvent {
                kind: "http".to_string(),
                body: to_raw_value(&self.0.body),
                raw_string: self.0.metadata.raw_string,
                headers,
                query,
                method: (&self.0.metadata.method).try_into()?,
                route: route_path,
                path: called_path,
                params,
            }),
        );
        Ok(PushArgsOwned { args, extra: None })
    }
}

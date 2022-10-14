use ::tracing::{field, Metadata, Span};
use ::tracing_subscriber::{
    filter::filter_fn,
    fmt::{format, Layer},
    prelude::*,
    EnvFilter,
};
use hyper::Response;
use tower_http::trace::{MakeSpan, OnResponse};
use tracing::Level;

#[derive(Clone)]
pub struct MyOnResponse {}

impl<B> OnResponse<B> for MyOnResponse {
    fn on_response(
        self,
        response: &Response<B>,
        latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        tracing::info!(
            latency = latency.as_millis(),
            status = response.status().as_u16(),
            "response"
        )
    }
}

#[derive(Clone)]
pub struct MyMakeSpan {}

impl<B> MakeSpan<B> for MyMakeSpan {
    fn make_span(&mut self, request: &hyper::Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            username = field::Empty,
            workspace_id = field::Empty,
            email = field::Empty,
        )
    }
}

fn json_layer<S>() -> Layer<S, format::JsonFields, format::Format<format::Json>> {
    tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_span_list(false)
        .with_current_span(true)
}

fn compact_layer<S>() -> Layer<S, format::DefaultFields, format::Format<format::Compact>> {
    tracing_subscriber::fmt::layer().compact()
}

fn filter_metadata(meta: &Metadata) -> bool {
    meta.target().starts_with("windmill")
}

pub fn initialize_tracing() {
    let tokio_console = std::env::var("TOKIO_CONSOLE")
        .map(|x| x == "true")
        .unwrap_or(false);
    let json_fmt = std::env::var("JSON_FMT")
        .map(|x| x == "true")
        .unwrap_or(false);

    let env_filter = EnvFilter::from_default_env();
    let env_filter = env_filter.add_directive(Level::INFO.into());

    let nenv_filter = if tokio_console {
        env_filter
            .add_directive("runtime=trace".parse().unwrap())
            .add_directive("tokio=trace".parse().unwrap())
    } else {
        env_filter
    };
    let ts_base = tracing_subscriber::registry().with(nenv_filter);

    match (json_fmt, tokio_console) {
        (true, true) => ts_base
            .with(json_layer().with_filter(filter_fn(filter_metadata)))
            .with(console_subscriber::spawn())
            .init(),
        (true, false) => ts_base
            .with(json_layer().with_filter(filter_fn(filter_metadata)))
            .init(),
        (false, true) => ts_base
            .with(compact_layer().with_filter(filter_fn(filter_metadata)))
            .with(console_subscriber::spawn())
            .init(),
        _ => ts_base
            .with(compact_layer().with_filter(filter_fn(filter_metadata)))
            .init(),
    }
}

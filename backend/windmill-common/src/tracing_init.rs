/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use tracing::Metadata;
use tracing_subscriber::{
    filter::filter_fn,
    fmt::{format, Layer},
    prelude::*,
    EnvFilter,
};

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

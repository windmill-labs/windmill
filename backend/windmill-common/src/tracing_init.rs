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
    let style = std::env::var("RUST_LOG_STYLE").unwrap_or_else(|_| "auto".into());
    let json_fmt = std::env::var("JSON_FMT")
        .map(|x| x == "true")
        .unwrap_or(false);

    let env_filter = EnvFilter::from_default_env();

    let ts_base = tracing_subscriber::registry().with(env_filter);

    match json_fmt {
        true => ts_base
            .with(
                json_layer()
                    .flatten_event(true)
                    .with_filter(filter_fn(filter_metadata)),
            )
            .init(),
        false => ts_base
            .with(
                compact_layer()
                    .with_ansi(style.to_lowercase() != "never")
                    .with_filter(filter_fn(filter_metadata)),
            )
            .init(),
    }
}

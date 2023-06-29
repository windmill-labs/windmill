/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use tracing_subscriber::{
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

pub fn initialize_tracing() {
    let style = std::env::var("RUST_LOG_STYLE").unwrap_or_else(|_| "auto".into());
    let json_fmt = std::env::var("JSON_FMT")
        .map(|x| x == "true")
        .unwrap_or(false);

    if std::env::var("RUST_LOG").is_ok_and(|x| x == "debug" || x == "info") {
        std::env::set_var(
            "RUST_LOG",
            &format!("windmill={}", std::env::var("RUST_LOG").unwrap()),
        )
    }

    let env_filter = EnvFilter::from_default_env();

    let ts_base = tracing_subscriber::registry().with(env_filter);

    match json_fmt {
        true => ts_base.with(json_layer().flatten_event(true)).init(),
        false => ts_base
            .with(compact_layer().with_ansi(style.to_lowercase() != "never"))
            .init(),
    }
}

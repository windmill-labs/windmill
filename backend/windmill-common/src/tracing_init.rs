/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use const_format::concatcp;
use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
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

pub const LOGS_SERVICE: &str = "logs/services/";
pub const TMP_WINDMILL_LOGS_SERVICE: &str = concatcp!("/tmp/windmill/", LOGS_SERVICE);

pub fn initialize_tracing() -> WorkerGuard {
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
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    let hostname = crate::utils::hostname();

    std::fs::create_dir_all(TMP_WINDMILL_LOGS_SERVICE).unwrap();
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::MINUTELY)
        .filename_prefix(format!("{}.log", hostname))
        .max_log_files(20)
        .build(TMP_WINDMILL_LOGS_SERVICE)
        .expect("Can build tracing file appender");

    let (log_file_writer, _guard) = NonBlockingBuilder::default()
        .lossy(false)
        .finish(file_appender);
    let stdout_and_log_file_writer = std::io::stdout.and(log_file_writer);

    let ts_base = tracing_subscriber::registry().with(env_filter);

    #[cfg(feature = "loki")]
    let ts_base = {
        let (layer, task) = tracing_loki::builder()
            .build_url(reqwest::Url::parse("http://127.0.0.1:3100").unwrap())
            .expect("build loki url");
        tokio::spawn(task);
        ts_base.with(layer)
    };

    match json_fmt {
        true => ts_base
            .with(
                json_layer()
                    .with_writer(stdout_and_log_file_writer)
                    .flatten_event(true),
            )
            .init(),
        false => ts_base
            .with(
                compact_layer()
                    .with_writer(stdout_and_log_file_writer)
                    .with_ansi(style.to_lowercase() != "never")
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(false),
            )
            .init(),
    }
    _guard
}

#[cfg(feature = "flamegraph")]
use tracing_flame::FlameLayer;

#[cfg(feature = "flamegraph")]
pub fn setup_flamegraph() -> impl Drop {
    // let fmt_layer = Layer::default();

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    tracing_subscriber::registry()
        // .with(fmt_layer)
        .with(flame_layer)
        .init();
    _guard
}

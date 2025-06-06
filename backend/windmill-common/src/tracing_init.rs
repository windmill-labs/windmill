/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use const_format::concatcp;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tracing::{level_filters::LevelFilter, Event};
use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::layer::Context;
use tracing_subscriber::{
    filter::Targets,
    fmt::{format, Layer},
    prelude::*,
    EnvFilter,
};

use crate::utils::Mode;

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

lazy_static::lazy_static! {
    pub static ref JSON_FMT: bool = std::env::var("JSON_FMT").map(|x| x == "true").unwrap_or(false);
}

pub const LOGS_SERVICE: &str = "logs/services/";

pub const TMP_WINDMILL_LOGS_SERVICE: &str = concatcp!("/tmp/windmill/", LOGS_SERVICE);

pub fn initialize_tracing(
    hostname: &str,
    mode: &Mode,
    environment: &str,
) -> (WorkerGuard, crate::otel_oss::OtelProvider) {
    let style = std::env::var("RUST_LOG_STYLE").unwrap_or_else(|_| "auto".into());

    let rust_log_env = std::env::var("RUST_LOG");
    if rust_log_env
        .as_ref()
        .is_ok_and(|x| x == "debug" || x == "info")
    {
        std::env::set_var(
            "RUST_LOG",
            &format!("windmill={}", rust_log_env.as_ref().unwrap()),
        )
    } else if rust_log_env.as_ref().is_ok_and(|x| x == "sqlxdebug") {
        std::env::set_var("RUST_LOG", "windmill=debug,sqlx=debug");
    };

    let default_env_filter = if rust_log_env.is_ok_and(|x| x == "debug" || x == "sqlxdebug") {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let meter_provider = crate::otel_oss::init_meter_provider(mode, hostname, environment);

    #[cfg(all(feature = "otel", feature = "enterprise"))]
    let opentelemetry = crate::otel_oss::init_otlp_tracer(mode, hostname, environment)
        .map(|x| tracing_opentelemetry::layer().with_tracer(x));

    #[cfg(not(all(feature = "otel", feature = "enterprise")))]
    let opentelemetry: Option<EnvFilter> = None;

    let logs_bridge = crate::otel_oss::init_logs_bridge(&mode, hostname, environment);

    use tracing_appender::rolling::{RollingFileAppender, Rotation};

    let log_dir = format!("{}/{}/", TMP_WINDMILL_LOGS_SERVICE, hostname);
    std::fs::create_dir_all(&log_dir).unwrap();
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::MINUTELY)
        .filename_prefix(format!("{}.log", hostname))
        .max_log_files(20)
        .build(log_dir)
        .expect("Can build tracing file appender");

    let (log_file_writer, _guard) = NonBlockingBuilder::default()
        .lossy(false)
        .finish(file_appender);
    let stdout_and_log_file_writer = std::io::stdout.and(log_file_writer);

    // let job_logs_filter = tracing_subscriber::filter::Targets::new()
    //     .with_target("windmill:job_log", tracing::Level::TRACE);

    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::level_filters::LevelFilter::ERROR.into())
        .from_env_lossy();

    let ts_base = tracing_subscriber::registry().with(env_filter);

    match *JSON_FMT {
        true => ts_base
            .with(logs_bridge)
            .with(opentelemetry)
            // .with(env_filter2.add_directive("windmill:job_log=off".parse().unwrap()))
            .with(
                json_layer()
                    .with_writer(stdout_and_log_file_writer)
                    .flatten_event(true)
                    .with_filter(
                        Targets::new()
                            .with_target(
                                "windmill:job_log",
                                tracing::level_filters::LevelFilter::OFF,
                            )
                            .with_default(default_env_filter),
                    ),
            )
            .with(CountingLayer::new())
            .init(),
        false => ts_base
            .with(logs_bridge)
            .with(opentelemetry)
            // .with(env_filter2.add_directive("windmill:job_log=off".parse().unwrap()))
            .with(
                compact_layer()
                    .with_writer(stdout_and_log_file_writer)
                    .with_ansi(style.to_lowercase() != "never")
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(false)
                    .with_filter(
                        Targets::new()
                            .with_target(
                                "windmill:job_log",
                                tracing::level_filters::LevelFilter::OFF,
                            )
                            .with_default(default_env_filter),
                    ),
            )
            .with(CountingLayer::new())
            .init(),
    }
    (_guard, meter_provider)
}

lazy_static::lazy_static! {
    pub static ref LOG_COUNTING_BY_MIN: Arc<RwLock<HashMap<String, LogCounter>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug)]
pub struct LogCounter {
    pub non_error_count: usize,
    pub error_count: usize,
}

impl LogCounter {
    fn new() -> Self {
        LogCounter { non_error_count: 0, error_count: 0 }
    }
}

#[derive(Debug)]
struct CountingLayer {}

impl CountingLayer {
    pub fn new() -> Self {
        CountingLayer {}
    }
}

pub const LOG_TIMESTAMP_FMT: &str = "%Y-%m-%d-%H-%M";

impl<S> tracing_subscriber::Layer<S> for CountingLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();

        let date_str = chrono::Utc::now().format(LOG_TIMESTAMP_FMT).to_string();
        let counters = LOG_COUNTING_BY_MIN.write();
        if let Ok(mut counters) = counters {
            let counter = counters.entry(date_str).or_insert(LogCounter::new());
            if level == tracing::Level::ERROR {
                counter.error_count += 1;
            } else {
                counter.non_error_count += 1;
            }
        } else {
            println!("Error getting lock for log counting");
        }
    }
}

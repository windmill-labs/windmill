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
    pub static ref QUIET_MODE: bool = std::env::var("QUIET").map(|x| x == "true" || x == "1").unwrap_or(false);
}

/// Target name for verbose logs that should be filtered in quiet mode.
/// Use `tracing::info!(target: windmill_common::tracing_init::VERBOSE_TARGET, ...)` for logs that should be suppressed in quiet mode.
pub const VERBOSE_TARGET: &str = "windmill_verbose";

/// Creates a Targets filter that optionally filters out verbose logs when quiet mode is enabled.
fn create_targets_filter(default_env_filter: LevelFilter) -> Targets {
    let targets =
        Targets::new().with_target("windmill:job_log", tracing::level_filters::LevelFilter::OFF);

    if *QUIET_MODE {
        targets
            .with_target(VERBOSE_TARGET, tracing::level_filters::LevelFilter::OFF)
            .with_default(default_env_filter)
    } else {
        targets.with_default(default_env_filter)
    }
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
    let rust_log_stdout_env = std::env::var("RUST_LOG_STDOUT");

    if rust_log_env
        .as_ref()
        .is_ok_and(|x| x == "debug" || x == "info")
    {
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                &format!("windmill={}", rust_log_env.as_ref().unwrap()),
            )
        }
    } else if rust_log_env.as_ref().is_ok_and(|x| x == "sqlxdebug") {
        unsafe {
            std::env::set_var("RUST_LOG", "windmill=debug,sqlx=debug");
        }
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

    // let job_logs_filter = tracing_subscriber::filter::Targets::new()
    //     .with_target("windmill:job_log", tracing::Level::TRACE);

    // Create the base filter for file writer (always uses RUST_LOG)
    let file_env_filter = EnvFilter::builder()
        .with_default_directive(tracing::level_filters::LevelFilter::ERROR.into())
        .from_env_lossy();

    // Create the filter for stdout (uses RUST_LOG_STDOUT if available, otherwise RUST_LOG)
    let stdout_env_filter = if rust_log_stdout_env.is_ok() {
        // Temporarily set RUST_LOG to RUST_LOG_STDOUT value to parse it
        let original_rust_log = std::env::var("RUST_LOG").ok();
        unsafe {
            std::env::set_var("RUST_LOG", rust_log_stdout_env.unwrap());
        }
        let filter = EnvFilter::builder()
            .with_default_directive(tracing::level_filters::LevelFilter::ERROR.into())
            .from_env_lossy();
        unsafe {
            // Restore original RUST_LOG
            match original_rust_log {
                Some(val) => std::env::set_var("RUST_LOG", val),
                None => std::env::remove_var("RUST_LOG"),
            }
        }
        filter
    } else {
        file_env_filter.clone()
    };

    // Create a common filter for OTEL logs bridge and tracing layer to respect RUST_LOG
    let otel_logs_filter = file_env_filter.clone();

    // Apply filter to the opentelemetry tracing layer to prevent debug events from being attached to spans
    #[cfg(all(feature = "otel", feature = "enterprise"))]
    let opentelemetry_filtered = {
        let otel_tracing_filter = file_env_filter.clone();
        opentelemetry.map(|layer| layer.with_filter(otel_tracing_filter))
    };

    #[cfg(not(all(feature = "otel", feature = "enterprise")))]
    let opentelemetry_filtered = opentelemetry;

    let base_layer = tracing_subscriber::registry()
        .with(logs_bridge.with_filter(otel_logs_filter))
        .with(opentelemetry_filtered);

    match *JSON_FMT {
        true => {
            // Stdout layer with its own filter
            let stdout_layer = json_layer()
                .with_writer(std::io::stdout)
                .flatten_event(true)
                .with_filter(stdout_env_filter)
                .with_filter(create_targets_filter(default_env_filter));

            // File layer with its own filter
            let file_layer = json_layer()
                .with_writer(log_file_writer)
                .flatten_event(true)
                .with_filter(file_env_filter)
                .with_filter(create_targets_filter(default_env_filter));

            base_layer
                .with(stdout_layer)
                .with(file_layer)
                .with(CountingLayer::new())
                .init()
        }
        false => {
            // Stdout layer with its own filter
            let stdout_layer = compact_layer()
                .with_writer(std::io::stdout)
                .with_ansi(style.to_lowercase() != "never")
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .with_filter(stdout_env_filter)
                .with_filter(create_targets_filter(default_env_filter));

            // File layer with its own filter
            let file_layer = compact_layer()
                .with_writer(log_file_writer)
                .with_ansi(false) // No ANSI codes in log files
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .with_filter(file_env_filter)
                .with_filter(create_targets_filter(default_env_filter));

            base_layer
                .with(stdout_layer)
                .with(file_layer)
                .with(CountingLayer::new())
                .init()
        }
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

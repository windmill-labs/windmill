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

pub fn initialize_tracing(hostname: &str) -> WorkerGuard {
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
            .with(CountingLayer::new())
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

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tracing::Event;
use tracing_subscriber::layer::Context;

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

// impl CountingLayer {
//     pub fn new() -> Self {
//         CountingLayer { counter: Arc::new(Mutex::new(LogCounter::new())) }
//     }

//     pub fn get_counts(&self) -> (usize, usize) {
//         let counter = self.counter.lock().unwrap();
//         (counter.non_error_count, counter.error_count)
//     }

//     pub fn reset_counts(&self) {
//         let mut counter = self.counter.lock().unwrap();
//         counter.reset();
//     }
// }

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

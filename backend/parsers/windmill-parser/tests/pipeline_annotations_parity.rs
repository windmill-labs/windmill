//! Parser-parity guard: `parse_pipeline_annotations` (Rust, drives deploy)
//! and `parsePipelineAnnotations` (TS, drives the live graph preview —
//! frontend/src/lib/components/assets/AssetGraph/parsePipelineAnnotations.ts)
//! must stay behaviorally identical, or the graph the user previews is not
//! the graph that deploys. Both implementations run the SAME fixture corpus:
//!
//!   tests/fixtures/pipeline_annotations.json
//!
//! The frontend counterpart is parsePipelineAnnotations.parity.test.ts.
//! When the annotation grammar changes, extend the corpus — a fixture that
//! passes on one side and fails on the other is exactly the drift this
//! exists to catch. Only the fields both parsers produce are compared
//! (join_mode / debounce_default are deploy-only, parsed solely in Rust).

use serde::Deserialize;
use windmill_parser::asset_parser::{
    parse_pipeline_annotations, AssetKind, PartitionKind, TriggerSpec,
};

#[derive(Deserialize)]
struct Fixture {
    name: String,
    code: String,
    expected: Expected,
}

#[derive(Deserialize)]
struct Expected {
    in_pipeline: bool,
    /// `kind:path`, in declaration order, deduped.
    asset_triggers: Vec<String>,
    native_triggers: Vec<String>,
    partition: Option<ExpectedPartition>,
    freshness: Option<String>,
    tag: Option<String>,
    retry: Option<ExpectedRetry>,
    // Default-on-absent so the pre-existing fixtures (which omit these) keep
    // deserializing; only fixtures exercising materialization set them.
    #[serde(default)]
    materialize: Option<ExpectedMaterialize>,
    #[serde(default)]
    unique_key: Option<String>,
    #[serde(default)]
    append: bool,
}

#[derive(Deserialize)]
struct ExpectedMaterialize {
    target_kind: String,
    target_path: String,
    #[serde(default)]
    wrap: bool,
}

#[derive(Deserialize)]
struct ExpectedPartition {
    kind: String,
    #[serde(default)]
    key: Option<String>,
    tz: Option<String>,
    format: Option<String>,
    start: Option<String>,
}

#[derive(Deserialize)]
struct ExpectedRetry {
    count: u32,
    delay: Option<String>,
}

fn kind_str(k: AssetKind) -> &'static str {
    match k {
        AssetKind::S3Object => "s3object",
        AssetKind::Resource => "resource",
        AssetKind::Ducklake => "ducklake",
        AssetKind::DataTable => "datatable",
        AssetKind::Volume => "volume",
    }
}

fn native_str(t: &TriggerSpec) -> Option<&'static str> {
    Some(match t {
        TriggerSpec::Asset { .. } => return None,
        TriggerSpec::Schedule => "schedule",
        TriggerSpec::Webhook => "webhook",
        TriggerSpec::Email => "email",
        TriggerSpec::Kafka => "kafka",
        TriggerSpec::Mqtt => "mqtt",
        TriggerSpec::Nats => "nats",
        TriggerSpec::Postgres => "postgres",
        TriggerSpec::Sqs => "sqs",
        TriggerSpec::Gcp => "gcp",
        TriggerSpec::DataUpload => "data_upload",
    })
}

#[test]
fn pipeline_annotation_fixtures_match() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/pipeline_annotations.json"))
            .expect("fixture corpus must deserialize");
    assert!(!fixtures.is_empty());

    for f in fixtures {
        let got = parse_pipeline_annotations(&f.code);
        let ctx = format!("fixture '{}'", f.name);

        assert_eq!(
            got.in_pipeline, f.expected.in_pipeline,
            "{ctx}: in_pipeline"
        );

        let asset_triggers: Vec<String> = got
            .triggers
            .iter()
            .filter_map(|t| match t {
                TriggerSpec::Asset { asset_kind, path, .. } => {
                    Some(format!("{}:{}", kind_str(*asset_kind), path))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            asset_triggers, f.expected.asset_triggers,
            "{ctx}: asset triggers"
        );

        let native: Vec<&str> = got.triggers.iter().filter_map(native_str).collect();
        assert_eq!(native, f.expected.native_triggers, "{ctx}: native triggers");

        match (&got.partition, &f.expected.partition) {
            (None, None) => {}
            (Some(p), Some(e)) => {
                let (kind, key) = match &p.kind {
                    PartitionKind::Daily => ("daily", None),
                    PartitionKind::Hourly => ("hourly", None),
                    PartitionKind::Weekly => ("weekly", None),
                    PartitionKind::Monthly => ("monthly", None),
                    PartitionKind::Dynamic { key } => ("dynamic", Some(key.clone())),
                };
                assert_eq!(kind, e.kind, "{ctx}: partition kind");
                assert_eq!(key, e.key, "{ctx}: partition key");
                assert_eq!(p.tz, e.tz, "{ctx}: partition tz");
                assert_eq!(p.format, e.format, "{ctx}: partition format");
                assert_eq!(p.start, e.start, "{ctx}: partition start");
            }
            (got, want) => panic!(
                "{ctx}: partition mismatch — got {:?}, want present={}",
                got,
                want.is_some()
            ),
        }

        assert_eq!(
            got.freshness.as_ref().map(|fr| fr.duration.clone()),
            f.expected.freshness,
            "{ctx}: freshness"
        );
        assert_eq!(got.tag, f.expected.tag, "{ctx}: tag");
        match (&got.retry, &f.expected.retry) {
            (None, None) => {}
            (Some(r), Some(e)) => {
                assert_eq!(r.count, e.count, "{ctx}: retry count");
                assert_eq!(r.delay, e.delay, "{ctx}: retry delay");
            }
            (got, want) => panic!(
                "{ctx}: retry mismatch — got {:?}, want present={}",
                got,
                want.is_some()
            ),
        }

        match (&got.materialize, &f.expected.materialize) {
            (None, None) => {}
            (Some(m), Some(e)) => {
                assert_eq!(
                    kind_str(m.target_kind),
                    e.target_kind,
                    "{ctx}: materialize kind"
                );
                assert_eq!(m.target_path, e.target_path, "{ctx}: materialize path");
                assert_eq!(m.wrap, e.wrap, "{ctx}: materialize wrap");
            }
            (got, want) => panic!(
                "{ctx}: materialize mismatch — got {:?}, want present={}",
                got,
                want.is_some()
            ),
        }
        assert_eq!(got.unique_key, f.expected.unique_key, "{ctx}: unique_key");
        assert_eq!(got.append, f.expected.append, "{ctx}: append");
    }
}

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
    // Default-on-absent so the pre-existing fixtures (which omit it) keep
    // deserializing; only fixtures exercising materialization set it.
    #[serde(default)]
    materialize: Option<ExpectedMaterialize>,
    // Snake_case `DataTest` serde shape (e.g. {"type":"unique","column":"x"}),
    // compared against `serde_json::to_value(got.data_tests)`. Absent === [].
    #[serde(default)]
    data_tests: Vec<serde_json::Value>,
    // Snake_case `ColumnLineage` serde shape (e.g. {"column":"x","inputs":
    // [{"from_kind":"datatable","from_path":"p","from_column":"c"}]}), compared
    // against `to_value(got.column_lineage)`. Absent === [].
    #[serde(default)]
    column_lineage: Vec<serde_json::Value>,
    // `// macros` marker (strict, alone on the line). Absent === false.
    #[serde(default)]
    macros: bool,
    // `// use <lib_path>` accumulation, declaration order, deduped. Absent === [].
    #[serde(default)]
    use_libs: Vec<String>,
}

#[derive(Deserialize)]
struct ExpectedMaterialize {
    target_kind: String,
    target_path: String,
    #[serde(default)]
    manual: bool,
    #[serde(default)]
    append: bool,
    #[serde(default)]
    unique_key: Option<String>,
    #[serde(default)]
    scd2: bool,
    #[serde(default)]
    track: Vec<String>,
    #[serde(default)]
    close_deleted: bool,
    // "warn" | "ignore"; absent === "warn" (the default).
    #[serde(default = "default_on_schema_change")]
    on_schema_change: String,
}

fn default_on_schema_change() -> String {
    "warn".to_string()
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
                assert_eq!(m.manual, e.manual, "{ctx}: materialize manual");
                assert_eq!(m.append, e.append, "{ctx}: materialize append");
                assert_eq!(m.unique_key, e.unique_key, "{ctx}: materialize key");
                assert_eq!(m.scd2, e.scd2, "{ctx}: materialize scd2");
                assert_eq!(m.track, e.track, "{ctx}: materialize track");
                assert_eq!(
                    m.close_deleted, e.close_deleted,
                    "{ctx}: materialize close_deleted"
                );
                let osc = match m.on_schema_change {
                    windmill_parser::asset_parser::OnSchemaChange::Warn => "warn",
                    windmill_parser::asset_parser::OnSchemaChange::Ignore => "ignore",
                };
                assert_eq!(
                    osc, e.on_schema_change,
                    "{ctx}: materialize on_schema_change"
                );
            }
            (got, want) => panic!(
                "{ctx}: materialize mismatch — got {:?}, want present={}",
                got,
                want.is_some()
            ),
        }

        let got_tests = serde_json::to_value(&got.data_tests).expect("data_tests serialize");
        assert_eq!(
            got_tests,
            serde_json::Value::Array(f.expected.data_tests.clone()),
            "{ctx}: data tests"
        );

        let got_lineage =
            serde_json::to_value(&got.column_lineage).expect("column_lineage serialize");
        assert_eq!(
            got_lineage,
            serde_json::Value::Array(f.expected.column_lineage.clone()),
            "{ctx}: column lineage"
        );

        assert_eq!(got.macros, f.expected.macros, "{ctx}: macros");
        assert_eq!(got.use_libs, f.expected.use_libs, "{ctx}: use_libs");
    }
}

//! Postgres `-- raw_output` envelope encoder.
//!
//! When a postgres script carries `-- raw_output`, `pg_executor` returns a
//! single JSON object of shape `{columns: [{name, oid, type_name}], rows:
//! [[text|null]]}` from the last statement, instead of the default
//! `[{col: val}]` row array. The shape gives a Postgres-protocol client (today
//! that's `wmill datatable serve`) everything it needs to build
//! `RowDescription` + `DataRow` messages without re-stringifying every JSON
//! value.
//!
//! All extra types and the per-row text conversion live here so
//! `pg_executor.rs` only has to dispatch on the annotation.

use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Serialize;
use serde_json::value::RawValue;
use tokio_postgres::Row;
use windmill_common::error::{self, to_anyhow, Error};
use windmill_common::worker::CLOUD_HOSTED;

use crate::pg_executor::{postgres_row_to_row_data_with_state, JSONValue, ResultFormatState};
use crate::MAX_RESULT_SIZE;

#[derive(Serialize, Clone, Debug)]
pub struct RawOutputColumn {
    pub name: String,
    pub oid: u32,
    pub type_name: String,
}

#[derive(Serialize, Debug)]
pub struct RawOutputEnvelope {
    pub columns: Vec<RawOutputColumn>,
    pub rows: Vec<Vec<Option<String>>>,
}

impl RawOutputEnvelope {
    /// Empty envelope returned when a raw_output query had no statements that
    /// produced rows (e.g. only `SET`). Surfaces to the client as
    /// `CommandComplete "SELECT 0"`.
    pub fn empty_raw_value() -> Box<RawValue> {
        RawValue::from_string("{\"columns\":[],\"rows\":[]}".to_string()).unwrap()
    }
}

/// Build a `RawOutputEnvelope` for one statement's worth of rows. Reuses the
/// existing JSON-cell formatter (numeric precision warning, interval/timetz
/// coercion, JSON columns…) and only post-processes its output into the text
/// form Postgres's wire protocol expects.
pub fn build_envelope(
    rows: Vec<Row>,
    format_state: &ResultFormatState,
    siz: &AtomicUsize,
) -> error::Result<RawOutputEnvelope> {
    let columns: Vec<RawOutputColumn> = rows
        .first()
        .map(|r| {
            r.columns()
                .iter()
                .map(|c| RawOutputColumn {
                    name: c.name().to_string(),
                    oid: c.type_().oid(),
                    type_name: c.type_().name().to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut text_rows: Vec<Vec<Option<String>>> = Vec::with_capacity(rows.len());
    for row in rows {
        let row_data = postgres_row_to_row_data_with_state(row, format_state).map_err(to_anyhow)?;
        let text_row: Vec<Option<String>> = columns
            .iter()
            .map(|c| {
                json_value_to_pg_text(row_data.get(&c.name).cloned().unwrap_or(JSONValue::Null))
            })
            .collect();
        siz.fetch_add(
            text_row
                .iter()
                .filter_map(|v| v.as_deref().map(str::len))
                .sum(),
            Ordering::Relaxed,
        );
        if *CLOUD_HOSTED {
            let total = siz.load(Ordering::Relaxed);
            if total > MAX_RESULT_SIZE * 4 {
                return Err(Error::ExecutionErr(format!(
                    "Query result too large for cloud (size = {} > {})",
                    total,
                    MAX_RESULT_SIZE * 4,
                )));
            }
        }
        text_rows.push(text_row);
    }

    Ok(RawOutputEnvelope { columns, rows: text_rows })
}

/// Extract the single envelope `Box<RawValue>` from the per-statement results
/// produced by `do_postgresql`, falling back to an empty envelope if no
/// statement contributed one (multi-statement query whose last item didn't
/// return rows).
pub fn extract_envelope_or_empty(results: Vec<Vec<Box<RawValue>>>) -> Box<RawValue> {
    results
        .into_iter()
        .last()
        .and_then(|v| v.into_iter().last())
        .unwrap_or_else(RawOutputEnvelope::empty_raw_value)
}

/// Convert a JSON value (as produced by `postgres_row_to_row_data_with_state`)
/// to the text shape Postgres's wire protocol expects in `DataRow`.
/// - SQL NULL → None (encoded as -1 length on the wire)
/// - JSON strings already carry the server's text repr
/// - Numbers/bools render with Postgres conventions (`t`/`f`)
/// - Arrays/objects fall back to JSON serialization (psql will display them
///   as JSON literals; not a valid pg array literal, but readable)
fn json_value_to_pg_text(v: JSONValue) -> Option<String> {
    match v {
        JSONValue::Null => None,
        JSONValue::String(s) => Some(s),
        JSONValue::Number(n) => Some(n.to_string()),
        JSONValue::Bool(true) => Some("t".to_string()),
        JSONValue::Bool(false) => Some("f".to_string()),
        JSONValue::Array(_) | JSONValue::Object(_) => Some(v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_value_to_pg_text_covers_every_kind() {
        // NULLs survive as None so the wire layer emits length -1.
        assert_eq!(json_value_to_pg_text(JSONValue::Null), None);
        // Strings already carry the server's text repr (e.g. "2024-01-15");
        // we deliberately strip the JSON quotes.
        assert_eq!(
            json_value_to_pg_text(JSONValue::String("hello".into())),
            Some("hello".to_string()),
        );
        // Numbers render without JSON wrapping.
        assert_eq!(
            json_value_to_pg_text(JSONValue::Number(serde_json::Number::from(42))),
            Some("42".to_string()),
        );
        // Booleans use Postgres's `t`/`f` text convention.
        assert_eq!(
            json_value_to_pg_text(JSONValue::Bool(true)),
            Some("t".to_string())
        );
        assert_eq!(
            json_value_to_pg_text(JSONValue::Bool(false)),
            Some("f".to_string())
        );
        // Arrays/objects fall back to JSON — psql will display them verbatim.
        assert_eq!(
            json_value_to_pg_text(serde_json::json!([1, 2, 3])),
            Some("[1,2,3]".to_string()),
        );
        assert_eq!(
            json_value_to_pg_text(serde_json::json!({"k": "v"})),
            Some("{\"k\":\"v\"}".to_string()),
        );
    }

    #[test]
    fn envelope_serializes_to_the_shape_the_cli_consumes() {
        // Lock in the JSON shape that the CLI's pg-wire bridge consumes.
        let envelope = RawOutputEnvelope {
            columns: vec![
                RawOutputColumn { name: "id".into(), oid: 23, type_name: "int4".into() },
                RawOutputColumn { name: "name".into(), oid: 25, type_name: "text".into() },
            ],
            rows: vec![
                vec![Some("1".into()), Some("alice".into())],
                vec![Some("2".into()), None],
            ],
        };
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "columns": [
                    {"name": "id", "oid": 23, "type_name": "int4"},
                    {"name": "name", "oid": 25, "type_name": "text"},
                ],
                "rows": [["1", "alice"], ["2", null]],
            }),
        );
    }

    #[test]
    fn extract_envelope_falls_back_to_empty_when_no_statement_produced_one() {
        let raw = extract_envelope_or_empty(vec![]);
        assert_eq!(raw.get(), "{\"columns\":[],\"rows\":[]}");

        let raw = extract_envelope_or_empty(vec![vec![]]);
        assert_eq!(raw.get(), "{\"columns\":[],\"rows\":[]}");
    }
}

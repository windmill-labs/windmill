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
use windmill_common::error::{self, to_anyhow};

use crate::pg_executor::{postgres_row_to_row_data_with_state, JSONValue, ResultFormatState};
use crate::sql_result_too_large_error;

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

/// What one converted row costs the worker. Containers count, not just the text
/// they hold: a row of NULLs still allocates a `Vec<Option<String>>` and one
/// `Option<String>` per column, so charging only for string bytes would let an
/// all-NULL or all-empty result grow without ever moving the total.
fn text_row_size(text_row: &[Option<String>]) -> usize {
    std::mem::size_of::<Vec<Option<String>>>()
        + text_row.len() * std::mem::size_of::<Option<String>>()
        + text_row
            .iter()
            .filter_map(|v| v.as_deref().map(str::len))
            .sum::<usize>()
}

/// Builds a `RawOutputEnvelope` one row at a time, charging each against the
/// caller's running total as it goes. Reuses the existing JSON-cell formatter
/// (numeric precision warning, interval/timetz coercion, JSON columns…) and only
/// post-processes its output into the text form Postgres's wire protocol expects.
///
/// Taking rows one at a time is what lets the caller drop each wire buffer as its
/// text form is appended, so the whole result is never resident twice.
pub struct RawOutputEnvelopeBuilder {
    columns: Vec<RawOutputColumn>,
    seen_first_row: bool,
    rows: Vec<Vec<Option<String>>>,
    max_result_size: usize,
}

impl RawOutputEnvelopeBuilder {
    /// Takes the caller's budget rather than reading it again: the envelope is
    /// serialized under the same cap once it is finished, and the two have to be
    /// the same number for that to mean anything.
    pub fn new(max_result_size: usize) -> Self {
        Self { columns: Vec::new(), seen_first_row: false, rows: Vec::new(), max_result_size }
    }

    pub fn push(
        &mut self,
        row: Row,
        format_state: &ResultFormatState,
        siz: &AtomicUsize,
    ) -> error::Result<()> {
        if !self.seen_first_row {
            self.seen_first_row = true;
            self.columns = row
                .columns()
                .iter()
                .map(|c| RawOutputColumn {
                    name: c.name().to_string(),
                    oid: c.type_().oid(),
                    type_name: c.type_().name().to_string(),
                })
                .collect();
        }

        let row_data = postgres_row_to_row_data_with_state(row, format_state).map_err(to_anyhow)?;
        let text_row: Vec<Option<String>> = self
            .columns
            .iter()
            .map(|c| {
                json_value_to_pg_text(row_data.get(&c.name).cloned().unwrap_or(JSONValue::Null))
            })
            .collect();
        siz.fetch_add(text_row_size(&text_row), Ordering::Relaxed);
        if siz.load(Ordering::Relaxed) > self.max_result_size {
            return Err(sql_result_too_large_error(self.max_result_size));
        }
        self.rows.push(text_row);
        Ok(())
    }

    pub fn finish(self) -> RawOutputEnvelope {
        RawOutputEnvelope { columns: self.columns, rows: self.rows }
    }
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

    /// The size cap is the only thing standing between a large raw_output result
    /// and the OOM killer, so a row has to cost something even when every cell is
    /// NULL — otherwise millions of them accumulate against a total that never
    /// moves.
    #[test]
    fn null_and_empty_cells_still_count_toward_the_cap() {
        assert!(text_row_size(&[None, None, None]) > 0);
        assert!(text_row_size(&[Some(String::new())]) > 0);
        assert!(text_row_size(&[None, None]) > text_row_size(&[None]));
        assert!(
            text_row_size(&[Some("abcde".to_string())]) > text_row_size(&[Some(String::new())])
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

//! Helpers for feeding S3Object files into native SQL scripts (PG, MSSQL, MySQL,
//! BigQuery, Snowflake) as bound parameters.
//!
//! Native SQL executors that already support `(s3object)` args call
//! [`fetch_s3object_as_json_text`]: it downloads the file via the authed-client S3
//! endpoint, infers the format from the object's extension, and returns a JSON-text
//! payload (`[{...}, {...}]`) that the user's SQL consumes via the dialect's JSON
//! parser (`OPENJSON`, `jsonb_to_recordset`, `JSON_TABLE`, ...).
//!
//! DuckDB does not go through this path — DuckDB's engine reads S3 natively, so its
//! executor binds the bare `s3://...` URI instead.

use anyhow::Context;
use windmill_common::client::AuthedClient;
use windmill_types::s3::S3Object;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputFormat {
    Json,
    Parquet,
    Csv,
}

fn detect_format(key: &str) -> InputFormat {
    let lower = key.to_ascii_lowercase();
    if lower.ends_with(".parquet") {
        InputFormat::Parquet
    } else if lower.ends_with(".csv") {
        InputFormat::Csv
    } else {
        // .json, .jsonl, .ndjson, no extension, anything else → treat as JSON text
        InputFormat::Json
    }
}

/// Fetch an [`S3Object`] and materialise it as a JSON text payload ready to bind as
/// a SQL parameter.
///
/// Behaviour by detected format:
/// - **Parquet** / **CSV**: decoded into a JSON array via the helpers in
///   `windmill-object-store` (require the `parquet` feature on the build).
/// - **JSON / JSONL**: returned as-is; if the bytes look like newline-delimited JSON
///   (multiple top-level values), they are rewrapped as a JSON array so user SQL can
///   uniformly assume an array shape.
pub async fn fetch_s3object_as_json_text(
    client: &AuthedClient,
    workspace_id: &str,
    obj: &S3Object,
) -> anyhow::Result<String> {
    let bytes = client
        .download_s3_file(workspace_id, &obj.s3, obj.storage.clone())
        .await
        .with_context(|| format!("Failed to download S3 object `{}`", obj.s3))?;

    match detect_format(&obj.s3) {
        InputFormat::Parquet => {
            windmill_object_store::decode_parquet_bytes_to_json_array(bytes).await
        }
        InputFormat::Csv => windmill_object_store::decode_csv_bytes_to_json_array(bytes).await,
        InputFormat::Json => normalise_json_or_jsonl(&bytes),
    }
}

/// Accept either:
/// - a single JSON value (object/array/scalar) — passed through verbatim;
/// - newline-delimited JSON (JSONL / NDJSON) — repackaged as a `[v1, v2, ...]` array.
///
/// The "is JSONL" heuristic is: bytes parse as a newline-separated sequence of
/// independently-valid JSON values, with at least one newline boundary. This avoids
/// misclassifying a pretty-printed JSON object that happens to contain newlines.
fn normalise_json_or_jsonl(bytes: &[u8]) -> anyhow::Result<String> {
    let text = std::str::from_utf8(bytes)
        .context("S3 input is not valid UTF-8 (expected JSON or JSONL)")?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok("[]".to_string());
    }

    if let Ok(values) = parse_jsonl(trimmed) {
        if values.len() > 1 {
            // genuine JSONL (≥2 records on separate lines): wrap as array.
            return Ok(serde_json::to_string(&values)?);
        }
        if values.len() == 1 {
            // Single record on a single line — could be either a one-record JSONL
            // or a single JSON value. Either way, returning the line as-is gives
            // the user the most flexibility (preserves shape).
            return Ok(values.into_iter().next().unwrap().get().to_string());
        }
    }

    // Fall back to validating the whole blob as a single JSON value.
    let parsed: Box<serde_json::value::RawValue> =
        serde_json::from_str(trimmed).context("S3 input could not be parsed as JSON or JSONL")?;
    Ok(parsed.get().to_string())
}

fn parse_jsonl(text: &str) -> anyhow::Result<Vec<Box<serde_json::value::RawValue>>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Box<serde_json::value::RawValue> = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSONL line: {}", truncate_for_log(line)))?;
        out.push(v);
    }
    Ok(out)
}

fn truncate_for_log(s: &str) -> String {
    const MAX_CHARS: usize = 80;
    if s.chars().count() <= MAX_CHARS {
        s.to_string()
    } else {
        let head: String = s.chars().take(MAX_CHARS).collect();
        format!("{}…", head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_by_extension() {
        assert_eq!(detect_format("path/file.parquet"), InputFormat::Parquet);
        assert_eq!(detect_format("path/file.PARQUET"), InputFormat::Parquet);
        assert_eq!(detect_format("path/file.csv"), InputFormat::Csv);
        assert_eq!(detect_format("path/file.json"), InputFormat::Json);
        assert_eq!(detect_format("path/file.jsonl"), InputFormat::Json);
        assert_eq!(detect_format("path/file.ndjson"), InputFormat::Json);
        assert_eq!(detect_format("path/with-no-ext"), InputFormat::Json);
    }

    #[test]
    fn jsonl_two_records_becomes_array() {
        let body = "{\"id\":1}\n{\"id\":2}\n";
        let out = normalise_json_or_jsonl(body.as_bytes()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, serde_json::json!([{"id":1},{"id":2}]));
    }

    #[test]
    fn jsonl_single_record_passes_through() {
        let body = "{\"id\":1}";
        let out = normalise_json_or_jsonl(body.as_bytes()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, serde_json::json!({"id":1}));
    }

    #[test]
    fn json_array_passes_through() {
        let body = "[{\"id\":1},{\"id\":2}]";
        let out = normalise_json_or_jsonl(body.as_bytes()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, serde_json::json!([{"id":1},{"id":2}]));
    }

    #[test]
    fn pretty_printed_json_object_is_not_misclassified_as_jsonl() {
        // Each line on its own is not valid JSON, so the JSONL parser bails and we
        // fall through to the single-value path.
        let body = "{\n  \"id\": 1,\n  \"name\": \"x\"\n}";
        let out = normalise_json_or_jsonl(body.as_bytes()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, serde_json::json!({"id":1,"name":"x"}));
    }

    #[test]
    fn empty_body_yields_empty_array() {
        let out = normalise_json_or_jsonl(b"").unwrap();
        assert_eq!(out, "[]");
    }

    #[test]
    fn malformed_payload_errors() {
        let err = normalise_json_or_jsonl(b"not valid json").unwrap_err();
        assert!(err.to_string().contains("could not be parsed"));
    }

    #[test]
    fn truncate_for_log_handles_multibyte_at_boundary() {
        // 80 narrow chars then a 4-byte emoji = byte index 80 falls inside the emoji.
        // Naive `&s[..80]` would panic; chars-aware truncation must not.
        let s = format!("{}{}", "a".repeat(80), "🦀");
        let out = truncate_for_log(&s);
        assert!(out.ends_with('…'));
    }
}

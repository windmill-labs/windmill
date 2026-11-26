//! The workspace's object storage (its large file storage and secondary storages), through
//! the endpoints of the S3 file browser: what the caller may list, read, delete or move there
//! is exactly what the browser lets them do.

use serde_json::{Map, Value};

use super::data::with_query;
use super::workspace::path_enc;
use super::{array, opt_str_arg, pretty, str_arg, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &[
    "list_s3_files",
    "read_s3_file",
    "delete_s3_file",
    "move_s3_file",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    let result = match name {
        "list_s3_files" => list_s3_files(ctx, args).await,
        "read_s3_file" => read_s3_file(ctx, args).await,
        "delete_s3_file" => delete_s3_file(ctx, args).await,
        "move_s3_file" => move_s3_file(ctx, args).await,
        _ => unreachable!("{name} is not a storage tool"),
    };
    // The object storage routes are mounted only on an Enterprise Edition server.
    result.map_err(|e| {
        if e == "404: Not Found" {
            "Object storage browsing needs a Windmill Enterprise Edition instance.".to_string()
        } else {
            e
        }
    })
}

const DEFAULT_MAX_KEYS: u64 = 100;
const MAX_KEYS: u64 = 500;
/// Bytes of a text file the preview endpoint is asked for.
const PREVIEW_BYTES: u64 = 32 * 1024;
/// A `read_s3_file` result as the model reads it, under the MCP runner's cap
/// (`MAX_TOOL_RESULT_BYTES`), which cuts a longer one mid-JSON.
const RESULT_BUDGET: usize = 48 * 1024;
/// Kept free for the note a preview adds once it is cut.
const NOTE_ROOM: usize = 2048;
const PARQUET_PREVIEW_ROWS: usize = 50;
/// The CSV preview endpoint parses at most this many rows, whatever length it is asked.
const CSV_PREVIEW_ROWS: usize = 20;
const MAX_CELL_CHARS: usize = 1000;
/// Anthropic's limit for one image; the server has no image library to downscale with.
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;
/// What the S3 file browser shows as an image.
const IMAGE_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
];

/// `None` for the primary storage, however it was named.
fn storage_arg(args: &Value) -> Option<&str> {
    opt_str_arg(args, "storage").filter(|s| !s.is_empty() && *s != "_default_")
}

/// The listing endpoint takes a folder as empty or `/`-terminated.
fn folder_prefix(prefix: Option<&str>) -> String {
    let prefix = prefix.unwrap_or_default().trim_start_matches('/');
    if prefix.is_empty() || prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{prefix}/")
    }
}

/// An S3 object as scripts and flows take it: `{ s3, storage? }`.
fn s3_object(key: &Value, storage: Option<&str>) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("s3".into(), key.clone());
    if let Some(storage) = storage {
        object.insert("storage".into(), storage.into());
    }
    object
}

async fn list_s3_files(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let storage = storage_arg(args);
    let prefix = folder_prefix(opt_str_arg(args, "prefix"));
    let page_token = opt_str_arg(args, "page_token");
    let max_keys = args
        .get("max_keys")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_MAX_KEYS)
        .clamp(1, MAX_KEYS)
        .to_string();
    let path = with_query(
        ctx.ws("job_helpers/list_stored_files_paged"),
        &[
            ("prefix", Some(&prefix)),
            ("max_keys", Some(&max_keys)),
            ("page_token", page_token),
            ("storage", storage),
        ],
    );
    let page = ctx.get(&path).await?;
    let storages = if prefix.is_empty() && page_token.is_none() {
        Some(
            ctx.get(&ctx.ws("workspaces/get_secondary_storage_names"))
                .await?,
        )
    } else {
        None
    };
    Ok(pretty(&listing(
        storage,
        &prefix,
        &page,
        storages,
        ctx.auth.is_admin,
    )))
}

fn listing(
    storage: Option<&str>,
    prefix: &str,
    page: &Value,
    storages: Option<Value>,
    is_admin: bool,
) -> Value {
    let folders: Vec<Value> = array(page.get("folders"))
        .iter()
        .filter_map(|f| f.get("prefix").cloned())
        .collect();
    let files: Vec<Value> = array(page.get("files"))
        .iter()
        .map(|f| {
            let mut file = s3_object(&f["key"], storage);
            file.insert("size".into(), f["size"].clone());
            file.insert("last_modified".into(), f["last_modified"].clone());
            Value::Object(file)
        })
        .collect();
    let empty = folders.is_empty() && files.is_empty();
    let mut out = Map::new();
    if let Some(storage) = storage {
        out.insert("storage".into(), storage.into());
    }
    out.insert("prefix".into(), prefix.into());
    out.insert("folders".into(), folders.into());
    out.insert("files".into(), files.into());
    if let Some(token) = page.get("next_page_token").filter(|t| t.is_string()) {
        out.insert("next_page_token".into(), token.clone());
    }
    if let Some(storages) = storages {
        out.insert("storages".into(), storages);
    }
    if empty {
        // The browser drops what the user may not list, so an empty folder and a refused
        // one look the same to anyone but an admin.
        let note = if is_admin {
            "Nothing here: the folder is empty or does not exist."
        } else {
            "Nothing listed: the folder is empty or does not exist, or this user may not list it. Listing needs workspace admin, storage set as public, or a storage permission rule granting it; tell the user that rather than concluding the storage is empty."
        };
        out.insert("note".into(), note.into());
    }
    Value::Object(out)
}

async fn read_s3_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let key = str_arg(args, "s3")?;
    let storage = storage_arg(args);
    let meta = ctx
        .get(&with_query(
            ctx.ws("job_helpers/load_file_metadata"),
            &[("file_key", Some(key)), ("storage", storage)],
        ))
        .await?;
    let size = meta["size_in_bytes"].as_u64().unwrap_or(0);
    let mut out = s3_object(&key.into(), storage);
    out.insert("size".into(), meta["size_in_bytes"].clone());
    out.insert("last_modified".into(), meta["last_modified"].clone());
    let download = with_query(
        ctx.ws("job_helpers/download_s3_file"),
        &[("file_key", Some(key)), ("storage", storage)],
    );
    if let Some(link) = ctx.link(&format!("/api{download}")) {
        out.insert("link".into(), link.into());
    }

    if key.to_lowercase().ends_with(".parquet") {
        let preview = ctx
            .get(&with_query(
                ctx.ws(&format!(
                    "job_helpers/load_parquet_preview/{}",
                    path_enc(key)
                )),
                &[
                    ("limit", Some(&PARQUET_PREVIEW_ROWS.to_string())),
                    ("storage", storage),
                ],
            ))
            .await?;
        out.insert("columns".into(), preview["columns"].clone());
        out.insert("column_types".into(), preview["column_types"].clone());
        fit_rows(&mut out, array(preview.get("rows")));
    } else if let Some(mime_type) = image_type(key) {
        if size > MAX_IMAGE_BYTES {
            out.insert(
                "image".into(),
                format!("not attached: larger than {} MB", MAX_IMAGE_BYTES >> 20).into(),
            );
        } else {
            let bytes = ctx
                .get_bytes(&with_query(
                    ctx.ws("job_helpers/load_image_preview"),
                    &[("file_key", Some(key)), ("storage", storage)],
                ))
                .await?;
            ctx.push_image(mime_type, &bytes);
            out.insert("image".into(), "attached".into());
        }
    } else {
        let mut length = size.min(PREVIEW_BYTES);
        let mut preview = text_preview(ctx, key, storage, size, length).await?;
        // The endpoint decodes text strictly, so a cut inside a multi-byte character fails
        // the whole preview; a UTF-8 character ends within the 3 bytes before it.
        while preview["msg"]
            .as_str()
            .is_some_and(|m| m.ends_with("File encoding is not supported"))
            && length < size
            && length + 3 > size.min(PREVIEW_BYTES)
        {
            length -= 1;
            preview = text_preview(ctx, key, storage, size, length).await?;
        }
        match preview["content"].as_str() {
            Some(content) => {
                out.insert("content_type".into(), preview["content_type"].clone());
                let csv_cut =
                    preview["content_type"] == "Csv" && content.lines().count() > CSV_PREVIEW_ROWS;
                fit_text(&mut out, content, length < size, csv_cut);
            }
            None => {
                out.insert("preview_unavailable".into(), preview["msg"].clone());
            }
        }
    }
    Ok(pretty(&Value::Object(out)))
}

async fn text_preview(
    ctx: &Ctx<'_>,
    key: &str,
    storage: Option<&str>,
    size: u64,
    length: u64,
) -> Result<Value, String> {
    ctx.get(&with_query(
        ctx.ws("job_helpers/load_file_preview"),
        &[
            ("file_key", Some(key)),
            ("storage", storage),
            ("file_size_in_bytes", Some(&size.to_string())),
            ("read_bytes_from", Some("0")),
            ("read_bytes_length", Some(&length.to_string())),
        ],
    ))
    .await
}

fn image_type(key: &str) -> Option<&'static str> {
    let ext = key.rsplit_once('.')?.1.to_lowercase();
    IMAGE_TYPES
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, mime_type)| *mime_type)
}

/// Bytes of `value` once pretty-printed `depth` levels deep in the result.
fn pretty_len(value: &Value, depth: usize) -> usize {
    let s = pretty(value);
    s.len() + s.matches('\n').count() * 2 * depth
}

/// The longest prefix of `text` whose JSON-escaped form fits in `max` bytes: a control
/// byte escapes to six.
fn cut_escaped(text: &str, max: usize) -> &str {
    let mut used = 0;
    for (i, c) in text.char_indices() {
        used += match c {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
            c if (c as u32) < 0x20 => 6,
            c => c.len_utf8(),
        };
        if used > max {
            return &text[..i];
        }
    }
    text
}

/// Room left in the result for one more field, its key and the note included.
fn room(out: &Map<String, Value>) -> usize {
    RESULT_BUDGET.saturating_sub(pretty_len(&Value::Object(out.clone()), 0) + NOTE_ROOM)
}

fn fit_text(out: &mut Map<String, Value>, content: &str, file_cut: bool, csv_cut: bool) {
    let shown = cut_escaped(content, room(out));
    let mut notes = vec![];
    if csv_cut {
        notes.push(format!("The first {CSV_PREVIEW_ROWS} rows only."));
    }
    if file_cut || shown.len() < content.len() {
        notes.push("Only the start of the file.".to_string());
    }
    out.insert("preview".into(), shown.into());
    if !notes.is_empty() {
        out.insert("note".into(), notes.join(" ").into());
    }
}

/// The Parquet rows that fit the result, with long strings cut at any depth; a first row
/// too wide on its own keeps its leading columns.
fn fit_rows(out: &mut Map<String, Value>, returned: &[Value]) {
    let mut room = room(out);
    let mut cut_cells = false;
    let mut rows = vec![];
    let mut left_out = vec![];
    for row in returned {
        let mut row = row.clone();
        cut_cells |= cut_strings(&mut row);
        // An array element sits two levels deep, behind its indent and separator.
        let cost = pretty_len(&row, 2) + 6;
        if cost <= room {
            room -= cost;
            rows.push(row);
            continue;
        }
        if rows.is_empty() {
            if let Value::Object(cells) = row {
                let mut kept = Map::new();
                for (column, cell) in cells {
                    let cost = column.len() + pretty_len(&cell, 3) + 12;
                    if left_out.is_empty() && cost <= room {
                        room -= cost;
                        kept.insert(column, cell);
                    } else {
                        left_out.push(column);
                    }
                }
                rows.push(Value::Object(kept));
            }
        }
        break;
    }
    let mut notes = vec![];
    if rows.len() < returned.len() || returned.len() == PARQUET_PREVIEW_ROWS {
        notes.push(match rows.len() {
            1 => "The first row only.".to_string(),
            n => format!("The first {n} rows only."),
        });
    }
    if cut_cells {
        notes.push(format!(
            "Text longer than {MAX_CELL_CHARS} characters is cut."
        ));
    }
    if !left_out.is_empty() {
        notes.push(format!(
            "{} columns left out of the row: {}.",
            left_out.len(),
            left_out.join(", ")
        ));
    }
    out.insert("rows".into(), rows.into());
    if !notes.is_empty() {
        out.insert("note".into(), notes.join(" ").into());
    }
}

/// Cut every string in `value` to `MAX_CELL_CHARS`; whether one was.
fn cut_strings(value: &mut Value) -> bool {
    match value {
        Value::String(s) => match s.char_indices().nth(MAX_CELL_CHARS) {
            Some((end, _)) => {
                s.truncate(end);
                true
            }
            None => false,
        },
        Value::Array(items) => items.iter_mut().fold(false, |cut, v| cut_strings(v) | cut),
        Value::Object(fields) => fields
            .values_mut()
            .fold(false, |cut, v| cut_strings(v) | cut),
        _ => false,
    }
}

fn in_storage(storage: Option<&str>) -> String {
    storage
        .map(|s| format!(" in storage `{s}`"))
        .unwrap_or_default()
}

async fn delete_s3_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let key = str_arg(args, "s3")?;
    let storage = storage_arg(args);
    ctx.delete(&with_query(
        ctx.ws("job_helpers/delete_s3_file"),
        &[("file_key", Some(key)), ("storage", storage)],
    ))
    .await?;
    Ok(format!("Deleted `{key}`{}.", in_storage(storage)))
}

async fn move_s3_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let key = str_arg(args, "s3")?;
    let dest = str_arg(args, "dest_s3")?;
    if key == dest {
        return Err("`dest_s3` is the file's current key: nothing to move".to_string());
    }
    let storage = storage_arg(args);
    // A GET on the browser's own route, which writes.
    ctx.get(&with_query(
        ctx.ws("job_helpers/move_s3_file"),
        &[
            ("src_file_key", Some(key)),
            ("dest_file_key", Some(dest)),
            ("storage", storage),
        ],
    ))
    .await?;
    Ok(format!("Moved `{key}` to `{dest}`{}.", in_storage(storage)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Each listed file must stay an S3 object the session's other tools take as is.
    #[test]
    fn listing_returns_s3_objects() {
        let page = json!({
            "restricted_access": false,
            "folders": [{ "prefix": "charts/2026/", "name": "2026" }],
            "files": [{ "key": "charts/a.csv", "name": "a.csv", "size": 34, "last_modified": "2026-11-25T15:11:58Z" }],
            "next_page_token": "sealed"
        });
        assert_eq!(
            listing(Some("archive"), "charts/", &page, None, false),
            json!({
                "storage": "archive",
                "prefix": "charts/",
                "folders": ["charts/2026/"],
                "files": [{ "s3": "charts/a.csv", "storage": "archive", "size": 34, "last_modified": "2026-11-25T15:11:58Z" }],
                "next_page_token": "sealed"
            })
        );
        assert_eq!(folder_prefix(Some("/charts")), "charts/");
        assert_eq!(folder_prefix(Some("/")), "");
    }

    /// The preview endpoints return whole rows whatever length was asked, and escaping
    /// grows text: what bounds the result is its serialized size, which the MCP runner cuts.
    #[test]
    fn previews_stay_within_the_result_budget() {
        let file = || s3_object(&json!("big/file"), None);
        let fits = |out: Map<String, Value>| {
            let text = pretty(&Value::Object(out));
            assert!(text.len() <= RESULT_BUDGET, "{} bytes", text.len());
            serde_json::from_str::<Value>(&text).unwrap()
        };

        let mut out = file();
        fit_text(
            &mut out,
            &"\u{1}".repeat(PREVIEW_BYTES as usize),
            false,
            false,
        );
        let shown = fits(out);
        assert_eq!(shown["note"], "Only the start of the file.");

        let wide: Map<String, Value> = (0..100)
            .map(|i| (format!("c{i}"), json!("x".repeat(1000))))
            .collect();
        let mut out = file();
        fit_rows(
            &mut out,
            &[Value::Object(wide.clone()), Value::Object(wide)],
        );
        let shown = fits(out);
        assert_eq!(shown["rows"].as_array().unwrap().len(), 1);
        assert!(shown["note"].as_str().unwrap().contains("columns left out"));

        let nested = json!({ "id": 1, "tags": ["y".repeat(1_000_000)] });
        let mut out = file();
        fit_rows(&mut out, &[nested]);
        let shown = fits(out);
        assert_eq!(
            shown["rows"][0]["tags"][0].as_str().unwrap().len(),
            MAX_CELL_CHARS
        );
    }
}

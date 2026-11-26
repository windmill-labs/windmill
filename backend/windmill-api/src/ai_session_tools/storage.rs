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
/// Bytes of a text file handed to the model, under the MCP runner's result limit.
const PREVIEW_BYTES: u64 = 32 * 1024;
const PARQUET_PREVIEW_ROWS: usize = 50;
const MAX_CELL_CHARS: usize = 1000;

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
        let returned = array(preview.get("rows"));
        let (rows, cut_cells) = bounded_rows(returned);
        out.insert("columns".into(), preview["columns"].clone());
        out.insert("column_types".into(), preview["column_types"].clone());
        let mut notes = vec![];
        if rows.len() < returned.len() || returned.len() == PARQUET_PREVIEW_ROWS {
            notes.push(format!("The first {} rows only.", rows.len()));
        }
        if cut_cells {
            notes.push(format!(
                "Text cells longer than {MAX_CELL_CHARS} characters are cut."
            ));
        }
        out.insert("rows".into(), rows.into());
        if !notes.is_empty() {
            out.insert("note".into(), notes.join(" ").into());
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
                // A CSV preview is the endpoint's parsed rows, whatever length was asked.
                let shown = cut_utf8(content, PREVIEW_BYTES as usize);
                out.insert("content_type".into(), preview["content_type"].clone());
                out.insert("preview".into(), shown.into());
                if shown.len() < content.len() || length < size {
                    out.insert("note".into(), "Only the start of the file.".into());
                }
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

fn cut_utf8(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Parquet rows within the preview budget, with long text cells cut; also whether a cell was.
fn bounded_rows(rows: &[Value]) -> (Vec<Value>, bool) {
    let mut kept = vec![];
    let mut cut_cells = false;
    // the array's brackets, then each row and its separator
    let mut bytes = 2;
    for row in rows {
        let mut row = row.clone();
        if let Value::Object(cells) = &mut row {
            for cell in cells.values_mut() {
                if let Value::String(s) = cell {
                    if let Some((end, _)) = s.char_indices().nth(MAX_CELL_CHARS) {
                        s.truncate(end);
                        cut_cells = true;
                    }
                }
            }
        }
        bytes += row.to_string().len() + 1;
        if bytes > PREVIEW_BYTES as usize && !kept.is_empty() {
            break;
        }
        kept.push(row);
    }
    (kept, cut_cells)
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

    /// The endpoints return whole rows whatever size was asked, so the tool must bound them.
    #[test]
    fn previews_stay_within_the_byte_budget() {
        let long = "é".repeat(PREVIEW_BYTES as usize);
        let cut = cut_utf8(&long, PREVIEW_BYTES as usize + 1);
        assert_eq!(cut.len(), PREVIEW_BYTES as usize);

        let rows: Vec<Value> = (0..50).map(|i| json!({ "id": i, "text": long })).collect();
        let (kept, cut_cells) = bounded_rows(&rows);
        assert!(cut_cells);
        assert!(kept.len() < rows.len());
        assert!(Value::from(kept).to_string().len() <= PREVIEW_BYTES as usize);
    }
}

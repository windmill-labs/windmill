# Object Storage CLI

`wmill object-storage` (alias `wmill s3`) exposes the workspace's object storage (S3-compatible: AWS S3, MinIO, GCS, R2, Azure Blob) over the per-workspace `/job_helpers/*` endpoints.

## Key concepts (not obvious from per-command --help)

- **`file_key` is the path inside the bucket** (e.g. `reports/2026-05/orders.csv`), not a Windmill path. Do NOT pass `u/...` or `f/...` here — those are Windmill paths to scripts/flows/resources, unrelated to objects in the bucket.
- **Scope is the active workspace.** Object storage is configured per-workspace (default storage + optional secondary storages). Switching workspaces switches which bucket the commands target.
- **`--storage <name>` targets a secondary storage** configured on the workspace. Omit it to use the workspace's default object storage. Use `wmill object-storage list` to discover configured storages.
- **`preview` vs `download`**: `preview` returns a peek (CSV first rows, text content, or a byte slice via `--bytes-from`/`--bytes-length`) without writing to disk. Use `download` when you want the full file on disk.

## Choosing a subcommand

- Look at what's there: `wmill object-storage files [prefix]` (alias `ls`) — paginated, use `--marker` to continue.
- Inspect one file: `wmill object-storage info <file_key>` for size/mime/last-modified, `wmill object-storage preview <file_key>` for content peek.
- Move data in: `wmill object-storage upload <local_path> <file_key>` — set `--content-type` if the receiver cares (e.g. `text/csv`).
- Move data out: `wmill object-storage download <file_key> [output_path]` — `--stdout` to pipe.
- Reorganize: `wmill object-storage move <src> <dest>` (same storage), `wmill object-storage delete <file_key>` (interactive confirm unless `--yes`).

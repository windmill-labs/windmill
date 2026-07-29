# Vendored object_store

Vendored copy of `apache/arrow-rs-object-store` at rev
`36752c975d4f29e20b57c91f81a10872dcd48ae7` (the rev this workspace previously
pinned via git), plus three Windmill commits — mirrored at
https://github.com/diegoimbert/arrow-rs-object-store/tree/windmill/list-delimited-page
(head `6a63a59ed5ee90c4e8e2bf502779437baf7aa806`):

- `fix: use runtime format widths accepted by recent rustc` (test-only)
- `feat: add ObjectStore::list_delimited_page for bounded, resumable delimited listing`
- `fix: saturate the page bound and disambiguate colliding entry names in the
  default list_delimited_page`
- `perf: stat only the returned page in LocalFileSystem's delimited paging`

`list_delimited_page` is the reason for the fork: one bounded page of a
delimited listing with an opaque continuation token, served natively on
S3/GCS/Azure. It is a candidate for upstreaming; if it lands, this directory
goes away and the dependency returns to the upstream pin.

To update: rebase the commits above onto the new upstream rev, rerun the crate
test suite (`cargo test -p object_store --lib`), and replace this directory.

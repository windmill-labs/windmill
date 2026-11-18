# Rich results

Windmill renders a script's or flow's return value according to its shape: on run pages, in the
script and flow editors, in app result displays and in AI chat run cards. Anything that matches none
of the shapes below is shown as JSON.

## Shapes

- `{ "table-row-object": [{...}, ...] }`, or a bare array of objects: a table whose columns are the
  union of the objects' keys.
- `{ "table-col": { "col": [...], ... } }`, or a bare object of equal-length arrays: a table built
  column by column.
- `{ "table-row": [["col", ...], [v, ...], ...] }`: a table whose first row holds the column names.
- `{ "chart": {...} }`: a bar, line, pie or scatter chart drawn from rows (schema below). A `chart`
  value the schema rejects is shown as JSON, not as a broken chart.
- `{ "markdown": "..." }` (or `md`): rendered markdown.
- `{ "html": "..." }`: sanitized HTML.
- `{ "png": "<base64>" }`, likewise `jpeg`, `gif`, `svg`: an image. `{ "pdf": "<base64>" }`: a PDF.
- `{ "file": "<base64>", "filename": "report.csv" }`: a download.
- `{ "map": { "lat": 0, "lon": 0, "zoom": 3, "markers": [{ "lat": 0, "lon": 0, "title": "..." }] } }`:
  a map.
- `{ "s3": "path/to/key" }`: an object in the workspace storage, previewed by type.
- `{ "render_all": [ ... ] }`: each item rendered in turn with the rules above.

Each shape is an object with that one key (plus `filename` for a file, and an optional `storage` for
`s3`). A result carrying any other key beside it is shown as JSON.

## Choosing one

A chart answers a question at a glance: a trend over time (line), a comparison across categories
(bar), parts of a whole with few parts (pie), or a relation between two numbers (scatter). A table
answers lookups and holds the exact values. When both help, return both in a `render_all`: the chart
first, then the table of the same rows.

Keep a chart to what it shows well: at most 8 series. A pie with more than 8 slices keeps the 7
largest and folds the rest into "Other", so prefer a bar chart for many categories. Aggregate before
charting: a chart of thousands of raw points says less than one of the totals per day.

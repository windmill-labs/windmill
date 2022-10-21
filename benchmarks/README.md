# Benchmarks

This folder includes a small deno/ts utility to benchmark execution of jobs & flows.

```
Usage:   windmill-bench

Description:

  Run Benchmark to measure throughput of windmill.

Options:

  -h, --help                                  - Show this help.
  -V, --version                               - Show the version number for this program.
  --host                <url>                 - The windmill host to benchmark.                                                (Default: "http://127.0.0.1/")
  --workers             <workers>             - The number of workers to run at once.                                          (Default: 1)
  -s, --seconds         <seconds>             - How long to run the benchmark for (in seconds).                                (Default: 30)
  -e, --email           <email>               - The email to use to login.
  -p, --password        <password>            - The password to use to login.
  -t, --token           <token>               - The token to use when talking to the API server. Preferred over manual login.
  -w, --workspace       <workspace>           - The workspace to spawn scripts from.                                           (Default: "starter")
  -m, --metrics         <metrics>             - The url to scrape metrics from.                                                (Default: "http://localhost:8001/metrics")
  --export-json         <export_json>         - If set, exports will be into a JSON file.
  --export-csv          <export_csv>          - If set, exports will be into a csv file.
  --export-histograms   [histograms...]       - Mark metrics (without label) that are reported as histograms to export.
  --export-simple       [simple...]           - Mark metrics (without label) that are reported as simple values.
  --maximum-throughput  <maximum_throughput>  - Maximum number of jobs/flows to start in one second.                           (Default: Infinity)
  --use-flows                                 - Run flows instead of jobs.
  --histogram-buckets   [buckets...]          - Define what buckets to collect from histograms.                                (Default: [
                                                                                                                                  "+Inf",  "10",    "5",
                                                                                                                                  "2.5",   "2.5",   "1",
                                                                                                                                  "0.5",   "0.25",  "0.1",
                                                                                                                                  "0.05",  "0.025", "0.01",
                                                                                                                                  "0.005"
                                                                                                                                ])

Environment variables:

  WM_TOKEN      <token>      - The token to use when talking to the API server. Preferred over manual login.
  WM_WORKSPACE  <workspace>  - The workspace to spawn scripts from.
```

This will run a simple benchmark against localhost (the default admin email + password are set above), all execution is done in the "bench" workspace (as set via `--workspace`).

Metrics are exported to JSON will only include mean & stdev, histograms get one entry for each bucket.
CSV will include a full list of all values scraped.

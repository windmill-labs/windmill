Tracing with Tempo
==================

[Tempo](https://grafana.com/oss/tempo/) is an open-source distributed tracing system for monitoring and debugging microservices. It helps track requests across services, analyze latency, identify bottlenecks, and diagnose failures.

Key use cases include debugging production issues, monitoring performance, visualizing service dependencies, and optimizing system reliability. As Tempo supports the OpenTelemetry protocol, it can be used to collect traces from Windmill.

Follow the guide on [setting up Tempo](https://windmill.dev/docs/misc/guides/otel#setting-up-tempo) for more details.

## Setting up Tempo along with Windmill

Start all services by running:

```bash
docker-compose up -d
```

## Configuring Windmill to use Tempo via OpenTelemetry Collector

In the Windmill UI available at `http://localhost`, complete the initial setup and go to "Instances Settings" and "OTEL/Prom" tab and fill in the open telemetry collector endpoint and service name and toggle the "Tracing" and "Logging" options to send traces and logs to the `otel-collector:4317` port.

## Open the Tempo UI

The Tempo UI is a plugin available in Grafana, if hosted with the `docker-compose.yml` file above, Grafana will be available at `http://localhost:3000`. When running a script or workflow with Windmill, you will be able to see the traces in the Tempo UI and investigate them. 

## Searching for specific traces

To search/filter for a specific trace, for example a workflow, you can use the search function in the Tempo UI by filtering by tags set by Windmill.

The following tags are useful to filter for specific traces:

- `job_id`: The ID of the job
- `root_job`: The ID of the root job (flow)
- `parent_job`: The ID of the parent job (flow)
- `flow_step_id`: The ID of the step within the workflow
- `script_path`: The path of the script
- `workspace_id`: The name of the workspace
- `worker_id`: The ID of the worker
- `language`: The language of the script
- `tag`: The queue tag of the workflow

## Monitoring metrics with Tempo and Prometheus

Tempo can be used to generate time series for metrics of the collected traces. These time series can be used to compare the performance of individual steps within a workflow and their overall performance and relative contribution over time, as well as identify and troubleshoot issues and anomalies.

These metrics are exported to Prometheus and can be viewed in the Grafana UI in the Metrics Explorer. The generated metrics are:

- traces_spanmetrics_calls_total
- traces_spanmetrics_latency
- traces_spanmetrics_latency_bucket
- traces_spanmetrics_latency_count
- traces_spanmetrics_latency_sum
- traces_spanmetrics_size_total

## Viewing Logs with Loki

Logs from Windmill are sent to [Loki](https://grafana.com/oss/loki/), a log aggregation system that integrates seamlessly with Grafana. You can view and analyze these logs in the Grafana UI.

To access logs in Grafana:

1. Open the Grafana UI, typically available at `http://localhost:3000`.
2. Navigate to the "Explore" section.
3. Select the Loki data source.
4. Use the query editor to filter and search logs based on various labels and fields.

This setup allows you to correlate logs with traces, providing a comprehensive view of your system's behavior and aiding in troubleshooting and performance analysis.

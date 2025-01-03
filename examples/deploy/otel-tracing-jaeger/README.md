Tracing with Jaeger
===================

[Jaeger](https://www.jaegertracing.io/) is an open-source distributed tracing system for monitoring and debugging microservices. Originally developed by Uber, it helps track requests across services, analyze latency, identify bottlenecks, and diagnose failures.

Key use cases include debugging production issues, monitoring performance, visualizing service dependencies, and optimizing system reliability. As Jaeger supports the OpenTelemetry protocol, it can be used to collect traces from Windmill.

Follow the guide on [setting up Jaeger](https://windmill.dev/docs/misc/guides/otel#setting-up-jaeger) for more details.

## Setting up Jaeger along with Windmill

Start all services by running:

```bash
docker-compose up -d
```

## Configuring Windmill to use Jaeger

In the Windmill UI available at `http://localhost`, complete the initial setup and go to "Instances Settings" and "OTEL/Prom" tab and fill in the Jaeger endpoint `jaeger:4317` and toggle the Tracing option to send traces to Jaeger.

## Open the Jaeger UI

The Jaeger UI, if hosted with the `docker-compose.yml` file above, will be available at `http://localhost:16686`. When running a script or workflow with Windmill, you will be able to see the traces in the Jaeger UI and investigate them. This can be useful to understand the performance of a workflow and identify bottlenecks in the Windmill server or client.

## Searching for specific traces

To search/filter for a specific trace, for example a workflow, you can use the search function in the Jaeger UI by filtering by tags set by Windmill.

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

## Monitoring metrics with Jaeger

Jaeger can be used to generate time series for metrics of the collected traces. These time series can be used to compare the performance of individual steps within a workflow and their overall performance and relative contribution over time, as well as identify and troubleshoot issues and anomalies.

In the Jaeger UI, you will now be able to see metrics time series for the traces in the "Monitor" tab.

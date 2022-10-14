/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.2/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";

// type DataFrame = dfd.DataFrame;
type DataFrame = any;

async function login(
  config: api.Configuration,
  email: string,
  password: string
): Promise<string> {
  return await new windmill.UserApi(config).login({
    email: email,
    password: password,
  });
}

await new Command()
  .name("windmill-bench")
  .description("Run Benchmark to measure throughput of windmill.")
  .version("v0.0.0")
  .option("--host <url:string>", "The windmill host to benchmark.", {
    default: "http://127.0.0.1/",
  })
  .option("--workers <workers:number>", "The number of jobs to run at once.", {
    default: 1,
  })
  .option(
    "-s --seconds <seconds:number>",
    "How long to run the benchmark for (in seconds).",
    {
      default: 30,
    }
  )
  .option("-e --email <email:string>", "The email to use to login.")
  .option("-p --password <password:string>", "The password to use to login.")
  .env(
    "WM_TOKEN=<token:string>",
    "The token to use when talking to the API server. Preferred over manual login."
  )
  .option(
    "-t --token <token:string>",
    "The token to use when talking to the API server. Preferred over manual login."
  )
  .env(
    "WM_WORKSPACE=<workspace:string>",
    "The workspace to spawn scripts from."
  )
  .option(
    "-w --workspace <workspace:string>",
    "The workspace to spawn scripts from.",
    { default: "starter" }
  )
  .option("-m --metrics <metrics:string>", "The url to scrape metrics from.", {
    default: "http://localhost:8001/metrics",
  })
  .option(
    "--export-json <export_json:string>",
    "If set, exports will be into a JSON file."
  )
  .option(
    "--export-histograms [histograms...:string]",
    "Mark metrics (without label) that are reported as histograms to export."
  )
  .option(
    "--histogram-buckets [buckets...:string]",
    "Define what buckets to collect from histograms.",
    {
      default: [
        "+Inf",
        "10",
        "5",
        "2.5",
        "2.5",
        "1",
        "0.5",
        "0.25",
        "0.1",
        "0.05",
        "0.025",
        "0.01",
        "0.005",
      ],
    }
  )
  .option(
    "--export-simple [simple...:string]",
    "Mark metrics (without label) that are reported as simple values."
  )
  .arguments("[domain]")
  .action(
    async ({
      host,
      workers: num_workers,
      seconds,
      email,
      password,
      token,
      workspace,
      metrics,
      exportJson,
      exportHistograms,
      exportSimple,
      histogramBuckets,
    }) => {
      const metrics_worker = new Worker(
        new URL("./scraper.ts", import.meta.url).href,
        {
          type: "module",
        }
      );

      if (!Array.isArray(histogramBuckets)) {
        histogramBuckets = [];
      }

      if (!Array.isArray(exportHistograms)) {
        exportHistograms = [];
      }

      if (!Array.isArray(exportSimple)) {
        exportSimple = [];
      }

      metrics_worker.postMessage({
        exportHistograms,
        histogramBuckets,
        exportSimple,
        host: metrics,
      });

      console.log("collecting samples...");
      host = host.endsWith("/") ? host.substring(0, host.length - 1) : host;
      host = `${host}/api`;

      let config = {
        ...api.createConfiguration({
          baseServer: new api.ServerConfiguration(host, {}),
        }),
        workspace_id: workspace,
      };

      let final_token: string;
      if (!token) {
        if (email && password) {
          final_token = await login(config, email, password);
        } else {
          console.error("Token or email with password are required.");
          return;
        }
      } else {
        final_token = token;
      }

      config = {
        ...api.createConfiguration({
          baseServer: config.baseServer,
          authMethods: {
            bearerAuth: {
              tokenProvider: {
                getToken() {
                  return final_token;
                },
              },
            },
          },
        }),
        workspace_id: config.workspace_id,
      };

      const shared_config = {
        server: host,
        token: final_token,
        workspace_id: config.workspace_id,
      };

      let workers: Worker[] = new Array(num_workers);
      for (let i = 0; i < num_workers; i++) {
        workers[i] = new Worker(new URL("./worker.ts", import.meta.url).href, {
          type: "module",
        });
      }

      workers.forEach((worker, i) => {
        worker.postMessage({ ...shared_config, i: i });
      });

      await sleep(seconds);

      workers.forEach((worker) => {
        const l = (_ev: MessageEvent<any>) => {
          worker.removeEventListener("message", l);
          workers = workers.filter((w) => w != worker);
          worker.terminate();
        };
        worker.addEventListener("message", l);
        worker.postMessage("stop");
      });

      console.log("waiting for shutdown");
      while (workers.length > 0) {
        await sleep(0.1);
      }

      metrics_worker.postMessage("stop");
      console.log("waiting for metrics");
      const { columns, transfer_values } = await new Promise<{
        columns: string;
        transfer_values: ArrayBufferLike[];
      }>((resolve, _reject) => {
        metrics_worker.onmessage = (e) => {
          resolve(e.data);
          metrics_worker.terminate();
        };
      });
      const values = transfer_values.map((x) => new Float32Array(x));

      if (exportJson) {
        console.log("exporting mean & stddev to json");
        const obj: any = {};
        for (let i = 0; i < columns.length; i++) {
          const name = columns[i]!;
          const value = values[i]!;
          const mean = value.reduce((acc, e) => acc + e, 0) / values.length;
          const stddev =
            value.reduce((acc, e) => acc + (e - mean) ** 2) / values.length;
          obj[name] = { mean, stddev };
        }

        await Deno.writeTextFile(exportJson, JSON.stringify(obj));
      }

      console.log("done");
    }
  )
  .parse();

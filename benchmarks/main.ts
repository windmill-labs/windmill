/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.2/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.38.5/mod.ts";
import { Action } from "./action.ts";

async function login(email: string, password: string): Promise<string> {
  return await windmill.UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });
}

await new Command()
  .name("windmill-bench")
  .description("Run Benchmark to measure throughput of windmill.")
  .version("v0.0.0")
  .option("--host <url:string>", "The windmill host to benchmark.", {
    default: "http://127.0.0.1",
  })
  .option(
    "--workers <workers:number>",
    "The number of workers to run at once.",
    {
      default: 1,
    },
  )
  .option(
    "-s --seconds <seconds:number>",
    "How long to run the benchmark for (in seconds).",
    {
      default: 30,
    },
  )
  .option("--max <max:number>", "Maximum number of operations performed.")
  .option("-e --email <email:string>", "The email to use to login.")
  .option("-p --password <password:string>", "The password to use to login.")
  .env(
    "WM_TOKEN=<token:string>",
    "The token to use when talking to the API server. Preferred over manual login.",
  )
  .option(
    "-t --token <token:string>",
    "The token to use when talking to the API server. Preferred over manual login.",
  )
  .env(
    "WM_WORKSPACE=<workspace:string>",
    "The workspace to spawn scripts from.",
  )
  .option(
    "-w --workspace <workspace:string>",
    "The workspace to spawn scripts from.",
    { default: "starter" },
  )
  .option("-m --metrics <metrics:string>", "The url to scrape metrics from.", {
    default: "http://localhost:8001/metrics",
  })
  .option(
    "--export-json <export_json:string>",
    "If set, exports will be into a JSON file.",
  )
  .option(
    "--export-csv <export_csv:string>",
    "If set, exports will be into a csv file.",
  )
  .option(
    "--export-histograms [histograms...:string]",
    "Mark metrics (without label) that are reported as histograms to export.",
  )
  .option(
    "--export-simple [simple...:string]",
    "Mark metrics (without label) that are reported as simple values.",
  )
  .option(
    "--maximum-throughput <maximum_throughput:number>",
    "Maximum number of jobs/flows to start in one second.",
    {
      default: Infinity,
    },
  )
  .option("--use-flows", "Run flows instead of jobs.")
  .option("--custom <custom_path:string>", "Use custom actions during bench")
  .option(
    "--zombie-timeout",
    "The maximum time in ms to wait for jobs to complete.",
    {
      default: 90000,
    },
  )
  .option(
    "--continous",
    "Run the benchmark forever. This effectively disables metric collection & exports. No zombie jobs will be tracked.",
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
    },
  )
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
      exportCsv,
      exportHistograms,
      exportSimple,
      histogramBuckets,
      maximumThroughput,
      useFlows,
      zombieTimeout,
      continous,
      max,
      custom,
    }) => {
      windmill.setClient("", host);

      const custom_content: Action | undefined = custom
        ? JSON.parse(await Deno.readTextFile(custom))
        : undefined;

      if (!Array.isArray(histogramBuckets)) {
        histogramBuckets = [];
      }

      if (!Array.isArray(exportHistograms)) {
        exportHistograms = [];
      }

      if (!Array.isArray(exportSimple)) {
        exportSimple = [];
      }

      let metrics_worker: Worker;
      if (!continous) {
        metrics_worker = new Worker(
          new URL("./scraper.ts", import.meta.url).href,
          {
            type: "module",
          },
        );

        metrics_worker.postMessage({
          exportHistograms,
          histogramBuckets,
          exportSimple,
          host: metrics,
        });
      }

      console.log(
        "Started with options",
        JSON.stringify(
          {
            host,
            num_workers,
            seconds,
            email,
            workspace,
            metrics,
            exportJson,
            exportCsv,
            exportHistograms,
            exportSimple,
            maximumThroughput,
            useFlows,
            zombieTimeout,
          },
          null,
          4,
        ),
      );

      const config = {
        token: "",
        server: host,
        workspace_id: workspace,
      };

      let final_token: string;
      if (!token) {
        if (email && password) {
          final_token = await login(email, password);
        } else {
          console.error("Token or email with password are required.");
          return;
        }
      } else {
        final_token = token;
      }

      config.token = final_token;
      windmill.setClient(final_token, host);

      const per_worker_throughput = maximumThroughput / num_workers;
      const max_per_worker = max ? max / num_workers : undefined;
      const shared_config = {
        server: host,
        token: final_token,
        workspace_id: config.workspace_id,
        per_worker_throughput,
        max_per_worker,
        useFlows,
        continous,
        custom: custom_content,
      };

      let workers: Worker[] = new Array(num_workers);
      for (let i = 0; i < num_workers; i++) {
        workers[i] = new Worker(new URL("./worker.ts", import.meta.url).href, {
          type: "module",
        });
      }

      let start: number | undefined = undefined;

      const jobsSent = Array(num_workers).fill(0);
      const enc = (s: string) => new TextEncoder().encode(s);

      const updateState = setInterval(async () => {
        const elapsed = start ? Math.ceil((Date.now() - start) / 1000) : 0;
        const sum = jobsSent.reduce((a, b) => a + b, 0);
        const queue_length = (await (await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } },
        )).json()).database_length;
        await Deno.stdout.write(
          enc(
            `elapsed: ${elapsed}/${seconds} | jobs sent: ${
              JSON.stringify(
                jobsSent,
              )
            } (sum: ${sum} thr: ${
              (sum / elapsed).toFixed(
                2,
              )
            }) | queue: ${queue_length}                          \r`,
          ),
        );
      }, 100);

      workers.forEach((worker, i) => {
        worker.postMessage({ ...shared_config, i });
        worker.addEventListener("message", (evt: MessageEvent<any>) => {
          if (evt.data.type === "jobs_sent") {
            jobsSent[i] = evt.data.jobs_sent;
          }
        });
      });
      start = Date.now();

      console.log("collecting samples...");
      if (continous) {
        while (true) {
          await sleep(Infinity);
        }
      }

      await sleep(seconds);

      clearInterval(updateState);

      const sum = jobsSent.reduce((a, b) => a + b, 0);
      await Deno.stdout.write(
        enc(" ".padStart(30) + `\rduration: ${seconds} | jobs sent: ${sum}\n`),
      );

      const shutdown_start = Date.now();
      let zombie_jobs = 0;
      let incorrect_results = 0;
      workers.forEach((worker) => {
        const l = (evt: MessageEvent<any>) => {
          if (evt.data.type === "zombie_jobs") {
            zombie_jobs += evt.data.zombie_jobs;
            incorrect_results += evt.data.incorrect_results;
            worker.removeEventListener("message", l);
            workers = workers.filter((w) => w != worker);
            worker.terminate();
          }
        };
        worker.addEventListener("message", l);
        worker.postMessage(
          Number.isSafeInteger(zombieTimeout) ? zombieTimeout : 90000,
        );
      });

      console.log("waiting for shutdown");
      while (workers.length > 0) {
        await sleep(0.1);
      }
      const tts = (Date.now() - shutdown_start) / 1000;
      console.log("time to shutdown:", tts);
      console.log("throughput /s", sum / (seconds + tts));

      console.log("zombie jobs: ", zombie_jobs);
      console.log("incorrect results: ", incorrect_results);
      console.log(
        "queue length:",
        (await (await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } },
        )).json()).database_length,
      );

      metrics_worker!.postMessage("stop");
      console.log("waiting for metrics");
      const { columns, transfer_values } = await new Promise<{
        columns: string[];
        transfer_values: ArrayBufferLike[];
      }>((resolve, _reject) => {
        metrics_worker.onmessage = (e) => {
          resolve(e.data);
          metrics_worker.terminate();
        };
      });
      const values = transfer_values.map((x) => new Float32Array(x));

      if (exportJson) {
        console.log("exporting mean & stdev to json");
        const obj: any = {};
        for (let i = 0; i < columns.length; i++) {
          const name = columns[i]!;
          const value = values[i]!;
          const mean = value.reduce((acc, e) => acc + e, 0) / values.length;
          const stdev = Math.sqrt(
            value.reduce((acc, e) => acc + (e - mean) ** 2) / values.length,
          );
          obj[name] = { mean, stdev };
        }

        await Deno.writeTextFile(exportJson, JSON.stringify(obj));
      }

      if (exportCsv) {
        const f = await Deno.open(exportCsv, {
          write: true,
          create: true,
          truncate: true,
        });
        const encoder = new TextEncoder();
        const newline = new Uint8Array(1);
        newline[0] = 0x0a;
        await f.write(encoder.encode(columns.join(",")));
        await f.write(newline);

        for (let i = 0; i < values.length; i++) {
          await f.write(encoder.encode(values[i].join(",")));
          await f.write(newline);
        }

        f.close();
      }
      console.log("done");
    },
  )
  .parse();

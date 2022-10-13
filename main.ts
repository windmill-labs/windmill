/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.2/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";
import { writeCSVObjects } from "https://deno.land/x/csv@v0.7.5/mod.ts";
import { serve } from "https://deno.land/std@0.159.0/http/server.ts";

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
  .option("-j --jobs <jobs:number>", "The number of jobs to run at once.", {
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
  .option("-m --metrics <metrics:string>", "The url to scrape metrics from", {
    default: "http://localhost:8001/metrics",
  })
  .option(
    "-o --metrics-file <metrics_file:string>",
    "The csv file to write metrics results to.",
    {
      default: "./output.csv",
    }
  )
  .arguments("[domain]")
  .action(
    async ({
      host,
      jobs,
      seconds,
      email,
      password,
      token,
      workspace,
      metrics,
      metricsFile,
    }) => {
      const metrics_worker = new Worker(
        new URL("./scraper.ts", import.meta.url).href,
        {
          type: "module",
        }
      );
      const metrics_data: Map<string, string>[] = [];
      metrics_worker.onmessage = (evt) => {
        const data = evt.data as Map<string, string>;
        metrics_data.push(data);
      };

      metrics_worker.postMessage(metrics);

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

      const workers: Worker[] = new Array(jobs);
      for (let i = 0; i < jobs; i++) {
        workers[i] = new Worker(new URL("./worker.ts", import.meta.url).href, {
          type: "module",
        });
      }

      workers.forEach((worker, i) => {
        worker.addEventListener("message", (message) => {
          console.log("from_worker: ", message);
        });

        worker.postMessage({ ...shared_config, i: i });
      });

      await sleep(seconds);

      workers.forEach((worker) => {
        worker.terminate();
      });

      sleep(0.2);
      metrics_worker.terminate();

      console.log(
        "collected " + metrics_data.length + " samples. writing csv file..."
      );

      const header = metrics_data
        .flatMap((x) => Array.from(x.keys()))
        .filter((v, i, a) => a.indexOf(v) == i);
      const f = await Deno.open(metricsFile, {
        write: true,
        create: true,
        truncate: true,
      });
      await writeCSVObjects(
        f,
        metrics_data.map((x) => Object.fromEntries(x)),
        { header }
      );
      f.close();
    }
  )
  .parse();

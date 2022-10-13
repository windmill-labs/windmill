/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.2/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";
import { InfluxDB, Point, HttpError } from "npm:@influxdata/influxdb-client";
import { BucketsAPI, OrgsAPI } from "npm:@influxdata/influxdb-client-apis";

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
  .option("-m --metrics <metrics:string>", "The url to scrape metrics from.", {
    default: "http://localhost:8001/metrics",
  })
  .option("--influx-host <influx_host:string>", "The influx url to write to.")
  .option("--influx-token <influx_token:string>", "The influx token to use.")
  .option("--influx-org <influx_org:string>", "The influx org to write to.")
  .option(
    "--influx-bucket <influx_bucket:string>",
    "The influx bucket to write to. Everything in the bucket will be deleted!!"
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
      influxHost,
      influxToken,
      influxOrg,
      influxBucket,
    }) => {
      const metrics_worker = new Worker(
        new URL("./scraper.ts", import.meta.url).href,
        {
          type: "module",
        }
      );

      let writeApi: any;
      if (influxHost && influxToken && influxOrg && influxBucket) {
        const influxDB = new InfluxDB({
          url: influxHost,
          token: influxToken,
        });
        console.log("influxDB enabled");

        const orgsAPI = new OrgsAPI(influxDB);
        const organizations = await orgsAPI.getOrgs({ org: influxOrg });
        if (
          !organizations ||
          !organizations.orgs ||
          !organizations.orgs.length
        ) {
          console.error(`No organization named "${influxOrg}" found!`);
          return;
        }
        const orgID = organizations.orgs[0].id;

        const bucketsAPI = new BucketsAPI(influxDB);
        try {
          const buckets = await bucketsAPI.getBuckets({
            orgID,
            name: influxBucket,
          });
          if (buckets && buckets.buckets && buckets.buckets.length) {
            console.log(buckets.buckets);
            const bucketID = buckets.buckets[0].id;
            await bucketsAPI.deleteBucketsID({ bucketID });
          }
        } catch (e) {
          if (e instanceof HttpError && e.statusCode == 404) {
            // OK, bucket not found
          } else {
            throw e;
          }
        }

        await bucketsAPI.postBuckets({ body: { orgID, name: influxBucket } });

        writeApi = influxDB.getWriteApi(influxOrg, influxBucket);

        metrics_worker.onmessage = (evt) => {
          const data = evt.data as {
            name: string;
            help: string;
          } & (
            | {
                type: "COUNTER" | "GAUGE";
                metrics: [{ value: string; labels: { [key: string]: string } }];
              }
            | {
                type: "HISTOGRAM";
                metrics: [{ buckets: { [key: string]: number } }];
              }
          );

          if (data.type == "COUNTER" || data.type == "GAUGE") {
            data.metrics.forEach((p) => {
              let point = new Point(data.name);
              new Map(Object.entries(p.labels)).forEach((v, l) => {
                point = point.tag(l, v);
              });
              point = point.floatField("value", p.value);
              writeApi.writePoint(point);
            });
          } else if (data.type == "HISTOGRAM") {
            data.metrics.forEach((p) => {
              let point = new Point(data.name);
              new Map(Object.entries(p.buckets)).forEach((v, k) => {
                point = point.floatField(k, v);
              });
              writeApi.writePoint(point);
            });
          }
        };
      }

      metrics_worker.postMessage(metrics);

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

      if (writeApi) {
        await writeApi.close();
      }
      console.log("done");
    }
  )
  .parse();

/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.2/command/mod.ts";
import { string } from "https://deno.land/x/cliffy@v0.25.2/flags/types/string.ts";
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
  .option(
    "--export-json <export_json:string>",
    "If set, exports will be into a JSON file."
  )
  .option(
    "--exports [exports...:string]",
    "Mark metrics (without label) to export."
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
      exportJson,
      exports,
    }) => {
      const metrics_worker = new Worker(
        new URL("./scraper.ts", import.meta.url).href,
        {
          type: "module",
        }
      );

      const export_map: Map<
        string,
        Map<string, { val: number; samples: number }>
      > = new Map();
      // exports is true | string[] | undefined. the first condition checks that it's not undefined, the second condition checks it's the array
      if (Array.isArray(exports)) {
        exports.forEach((e) => {
          export_map.set(e, new Map());
        });
      }

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
      }

      metrics_worker.onmessage = (evt) => {
        const data = evt.data as {
          name: string;
          help: string;
        } & (
          | {
              type: "COUNTER" | "GAUGE";
              metrics: [{ value: string; labels: Record<string, string> }];
            }
          | {
              type: "HISTOGRAM";
              metrics: [{ buckets: Record<string, number> }];
            }
        );

        if (export_map.has(data.name)) {
          const c = export_map.get(data.name)!;
          if (data.type == "COUNTER" || data.type == "GAUGE") {
            const c2 = c.get("value") ?? { val: 0, samples: 0 };
            data.metrics.forEach((e) => {
              c2.val += Number(e.value);
              c2.samples++;
            });
            c.set("value", c2);
          } else if (data.type == "HISTOGRAM") {
            data.metrics.forEach((e) => {
              new Map(Object.entries(e.buckets)).forEach((v, b) => {
                const c2 = c.get(b) ?? { val: 0, samples: 0 };
                c2.val += Number(v);
                c2.samples++;
                c.set(b, c2);
              });
            });
          }
          export_map.set(data.name, c);
        }

        if (writeApi) {
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
        }
      };

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

      let workers: Worker[] = new Array(jobs);
      for (let i = 0; i < jobs; i++) {
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

      metrics_worker.terminate();

      if (writeApi) {
        await writeApi.close();
      }
      if (exportJson) {
        console.log("exporting to json");
        const final_map: Map<string, number | Map<string, number>> = new Map();
        export_map.forEach((v, k) => {
          if (v.size === 1) {
            const v2 = v.get("value")!;
            final_map.set(k, v2.val / v2.samples);
          } else {
            const m: Map<string, number> = new Map();
            v.forEach((v2, k2) => {
              const mean = v2.val / v2.samples;
              m.set(k2, mean);
            });
            final_map.set(k, m);
          }
        });
        const json = JSON.stringify(final_map, (_k, v) => {
          if (v instanceof Map) {
            return Object.fromEntries(v);
          }
          return v;
        });
        console.log(json);
        await Deno.writeTextFile(exportJson, json);
      }

      console.log("done");
    }
  )
  .parse();

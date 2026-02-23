import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { pickInstance } from "../instance/instance.ts";

type Data = {
  count: number;
  later: number;
  suspended: number;
  waiting: number;
  running: number;
  rps30s: string;
  rps5min: string;
  rps30min: string;
  rps24h: string;
}

type GlobalOptions = {
  instance?: string;
  baseUrl?: string;
};

    
function createRow(tag: string, data: Record<string, Data>) {
  if (data[tag]) {
    return;
  } else {
    data[tag] = {
      count: 0,
      waiting: 0,
      later: 0,
      running: 0,
      suspended: 0,
      rps30s: "",
      rps5min: "",
      rps30min: "",
      rps24h: "",
    }
  }
}
async function displayQueues(opts: GlobalOptions, workspace?: string) {
  const activeInstance = await pickInstance(opts, true);
  if (activeInstance) {
    try {
      const queuedJobs = await wmill.listQueue({workspace: workspace ?? 'admins', allWorkspaces: workspace === undefined, perPage: 100000});
      const jobCounts30s = await wmill.countJobsByTag({
        horizonSecs: 30,
        workspaceId: workspace,
      });
      const nowFromDb = new Date(await wmill.getDbClock());
      const jobCounts5min = await wmill.countJobsByTag({
        horizonSecs: 300,
        workspaceId: workspace,

      });
      const jobCounts30min = await wmill.countJobsByTag({
        horizonSecs: 1800,
        workspaceId: workspace,

      });
      const jobCounts24h = await wmill.countJobsByTag({
        horizonSecs: 86400,
        workspaceId: workspace,

      });

      const data: Record<string, Data> = {}

      for (const job of queuedJobs) {
        createRow(job.tag, data);
        const scheduledFor = new Date(job.scheduled_for ?? "");
        if (job.running) {
          if (job.suspend) {
            data[job.tag].suspended += 1;
          } else {
            data[job.tag].running += 1;
          }
        } else if (scheduledFor <= nowFromDb) {
          data[job.tag].waiting += 1;
        } else {
          data[job.tag].later += 1;
        } 
      }

      for (const count of jobCounts30s) {
        const tag = count.tag;
        createRow(tag, data);
        data[tag].rps30s = (count.count / 30).toFixed(3);
      }
      for (const count of jobCounts5min) {
        const tag = count.tag;
        createRow(tag, data);
        data[tag].rps5min = (count.count / 300).toFixed(3);
      }
      for (const count of jobCounts30min) {
        const tag = count.tag;
        createRow(tag, data);
        data[tag].rps30min = (count.count / 1800).toFixed(3);
      }

      for (const count of jobCounts24h) {
        const tag = count.tag;
        createRow(tag, data);
        data[tag].rps24h = (count.count / 86400).toFixed(3);
      }

      const table = new Table();
      table.header([
        "",
        "Running",
        "Waiting",
        "Later",
        "Suspended",
        "RPS (30s)",
        "RPS (5min)",
        "RPS (30min)",
        "RPS (24h)",
      ]);
      let body = []
      for (const tag in data) {
        body.push([tag, data[tag].running, data[tag].waiting, data[tag].later, data[tag].suspended, data[tag].rps30s, data[tag].rps5min, data[tag].rps30min, data[tag].rps24h]);
      }
      table.body(body).render();

    } catch (error) {
      log.error(`Failed to fetch queue metrics: ${error}`);
    }
  } else {
    log.info("No active instance found");
    log.info("Use 'wmill instance add' to add a new instance");
  }
}

const command = new Command()
  .description("List all queues with their metrics")
  .arguments("[workspace:string] the optional workspace to filter by (default to all workspaces)")
  .option(
    "--instance [instance]",
    "Name of the instance to push to, override the active instance"
  )
  .option(
    "--base-url [baseUrl]",
    "If used with --token, will be used as the base url for the instance"
  )
  .action(displayQueues as any);

export default command;

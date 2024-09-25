import { Command, Table } from "./deps.ts";
import { log } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { pickInstance } from "./instance.ts";

type Data = {
  count: number;
  waiting: number;
  running: number;
  rps30s: number;
  rps5min: number;
  rps30min: number;
  rps24h: number;
}

    
function createRow(tag: string, data: Record<string, Data>) {
  if (data[tag]) {
    return;
  } else {
    data[tag] = {
      count: 0,
      waiting: 0,
      running: 0,
      rps30s: 0,
      rps5min: 0,
      rps30min: 0,
      rps24h: 0,
    }
  }
}
async function displayQueues({ workspace }: { workspace?: string }) {
  const activeInstance = await pickInstance({}, true);

  if (activeInstance) {
    try {
      const queuedJobs = await wmill.listQueue({workspace: workspace ?? 'admins', allWorkspaces: workspace === undefined});
      const jobCounts30s = await wmill.countJobsByTag({
        horizonSecs: 30,
      });
      const jobCounts5min = await wmill.countJobsByTag({
        horizonSecs: 300,
      });
      const jobCounts30min = await wmill.countJobsByTag({
        horizonSecs: 1800,
      });
      const jobCounts24h = await wmill.countJobsByTag({
        horizonSecs: 86400,
      });
      const table = new Table();
      table.header([
        "Queue",
        "Jobs Waiting",
        "Jobs Running",
        "RPS (30s)",
        "RPS (5min)",
        "RPS (30min)",
        "RPS (24h)",
      ]);

      const data: Record<string, Data> = {}

      for (const job of queuedJobs) {
        createRow(job.queue, data);
        data[job.queue].waiting += 1;
      }

      for (const count of jobCounts30s) {
        const tag = count.tag;
        createRow(tag, data);
        data[tag].running = count.count;
      }

    } catch (error) {
      log.error("Failed to fetch queue metrics:", error);
    }
  } else {
    log.info("No active instance found");
    log.info("Use 'wmill instance add' to add a new instance");
  }
}

const command = new Command()
  .description("List all queues with their metrics")
  .arguments("[workspace:string] the optional workspace to filter by")
  .action(displayQueues);

export default command;

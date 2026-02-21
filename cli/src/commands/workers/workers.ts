import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "@std/log";
import * as wmill from "../../../gen/services.gen.ts";
import { pickInstance } from "../instance/instance.ts";

type GlobalOptions = {
  instance?: string;
  baseUrl?: string;
};


function toPercent(value: number | undefined): string {
  return value != undefined ? `${(value * 100).toFixed(1)}%` : '?%';
}

async function displayWorkers(opts: GlobalOptions) {
  const activeInstance = await pickInstance(opts, true);
  
  if (activeInstance) {
      const workerGroups = await wmill.listWorkerGroups();
      const workers = await wmill.listWorkers({
        pingSince: 10
      });

      const groupedWorkers = workerGroups.map(group => {

        
        const groupWorkers = workers.filter(worker => worker.worker_group === group.name);
        return {
          groupName: group.name,
          workers: groupWorkers
        };
      });

      // Add workers that don't belong to any worker group
      const ungroupedWorkers = workers.filter(worker => 
        !workerGroups.some(group => group.name === worker.worker_group)
      );
      
      if (ungroupedWorkers.length > 0) {
        ungroupedWorkers.forEach(worker => {
          const groupName = worker.worker_group || "Ungrouped";
          let group = groupedWorkers.find(g => g.groupName === groupName);
          if (!group) {
            group = { groupName, workers: [] };
            groupedWorkers.push(group);
          }
          group.workers.push(worker);
        });
      }

      // Sort groupedWorkers
      groupedWorkers.sort((a, b) => {
        // Always put 'default' first
        if (a.groupName === 'default') return -1;
        if (b.groupName === 'default') return 1;
        
        // Then sort by number of workers (descending order)
        return b.workers.length - a.workers.length;
      });

      groupedWorkers.forEach(group => {
        log.info(`\nWorker Group: ${group.groupName} (${group.workers.length} workers)`);
        if (group.workers.length === 0) {
          log.info("  No workers in this group");
        } else {

          new Table()
            .header(["Worker ID", "Host", "Queues",  "Jobs", "Occupancy rate 15s/5m/30m/ever)", "Last job", "Last Ping"])
            .padding(2)
            .border(true)
            .maxColWidth(30)
            .body(group.workers.map(worker => [
              worker.worker,
              worker.worker_instance,
              worker.custom_tags?.join(', ') || '',
              worker.jobs_executed,
              `${toPercent(worker.occupancy_rate_15s)}/${toPercent(worker.occupancy_rate_5m)}/${toPercent(worker.occupancy_rate_30m)}/${toPercent(worker.occupancy_rate)}`,
              
              worker.last_job_id ? worker.last_job_id + ' ' +worker.last_job_workspace_id : '',
              `${worker.last_ping}s ago`
            ]))
            .render();
        }
      });

  } else {
    log.info("No active instance found");
    log.info("Use 'wmill instance add' to add a new instance");
  }
}

const command = new Command()
  .description("List all workers grouped by worker groups")
  .option(
    "--instance [instance]",
    "Name of the instance to push to, override the active instance"
  )
  .option(
    "--base-url [baseUrl]",
    "If used with --token, will be used as the base url for the instance"
  )
  .action(displayWorkers as any);

export default command;

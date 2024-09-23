import { Command, setClient, Table } from "./deps.ts";
import { log } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { pickInstance } from "./instance.ts";

type GlobalOptions = {
  instance?: string;
  baseUrl?: string;
};



async function displayWorkers() {
  const activeInstance = await pickInstance({}, true);

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
            .header(["Worker ID", "Host", "Queues",  "Jobs", "Last job", "Last Ping"])
            .padding(2)
            .border(true)
            .maxColWidth(30)
            .body(group.workers.map(worker => [
              worker.worker,
              worker.worker_instance,
              worker.custom_tags?.join(', ') || '',
              worker.jobs_executed,
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
  .action(displayWorkers);

export default command;

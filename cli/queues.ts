import { Command, Table } from "./deps.ts";
import { log } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { pickInstance } from "./instance.ts";

async function displayQueues() {
  const activeInstance = await pickInstance({}, true);

  if (activeInstance) {
    try {
      const queueMetrics = await wmill.getQueueMetrics();
      const jobCounts5min = await wmill.countJobsByTag({
        horizon: 360,
        workspace: activeInstance.workspace,
      });
      
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
  .action(displayQueues);

export default command;

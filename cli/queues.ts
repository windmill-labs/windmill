import { Command, Table } from "./deps.ts";
import { log } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { pickInstance } from "./instance.ts";

async function displayQueues() {
  const activeInstance = await pickInstance({}, true);

  if (activeInstance) {
    try {
      const queueMetrics = await wmill.getQueueMetrics();
      
      const queues = queueMetrics.reduce((acc, metric) => {
        if (!acc[metric.id]) {
          acc[metric.id] = {
            id: metric.id,
            jobs24h: 0,
            jobs1h: 0,
            jobs5m: 0,
            waiting: 0,
            avgDelay24h: 0,
            avgDelay1h: 0,
            avgDelay5m: 0,
          };
        }
        
        metric.values.forEach(value => {
          const date = new Date(value.created_at);
          const now = new Date();
          const diffMinutes = (now.getTime() - date.getTime()) / (1000 * 60);
          
          if (diffMinutes <= 5) {
            acc[metric.id].jobs5m += value.value;
            acc[metric.id].avgDelay5m += value.value * diffMinutes;
          }
          if (diffMinutes <= 60) {
            acc[metric.id].jobs1h += value.value;
            acc[metric.id].avgDelay1h += value.value * diffMinutes;
          }
          if (diffMinutes <= 1440) {
            acc[metric.id].jobs24h += value.value;
            acc[metric.id].avgDelay24h += value.value * diffMinutes;
          }
        });
        
        if (acc[metric.id].jobs5m > 0) acc[metric.id].avgDelay5m /= acc[metric.id].jobs5m;
        if (acc[metric.id].jobs1h > 0) acc[metric.id].avgDelay1h /= acc[metric.id].jobs1h;
        if (acc[metric.id].jobs24h > 0) acc[metric.id].avgDelay24h /= acc[metric.id].jobs24h;
        
        return acc;
      }, {} as Record<string, any>);

      const workers = await wmill.listWorkers({ pingSince: 300 });

      // Get all unique tags/queues from workers
      const allTags = new Set<string>();
      workers.forEach(worker => {
        worker.custom_tags?.forEach(tag => allTags.add(tag));
      });

      // Add any queues from metrics that aren't in worker tags
      Object.keys(queues).forEach(queueId => allTags.add(queueId));

      // Create queue objects for any tags that don't have metrics
      allTags.forEach(tag => {
        if (!queues[tag]) {
          queues[tag] = {
            id: tag,
            jobs24h: 0,
            jobs1h: 0,
            jobs5m: 0,
            waiting: 0,
            avgDelay24h: 0,
            avgDelay1h: 0,
            avgDelay5m: 0,
          };
        }
      });
      const workerTags = workers.reduce((acc, worker) => {
        worker.custom_tags?.forEach(tag => {
          if (!acc[tag]) acc[tag] = 0;
          acc[tag]++;
        });
        return acc;
      }, {} as Record<string, number>);

      new Table()
        .header(["Queue", "Jobs 24h", "Jobs 1h", "Jobs 5m", "Waiting", "Avg Delay 24h", "Avg Delay 1h", "Avg Delay 5m", "Workers"])
        .padding(2)
        .border(true)
        .body(Object.values(queues).map(queue => [
          queue.id,
          queue.jobs24h.toString(),
          queue.jobs1h.toString(),
          queue.jobs5m.toString(),
          queue.waiting.toString(),
          `${queue.avgDelay24h.toFixed(2)}m`,
          `${queue.avgDelay1h.toFixed(2)}m`,
          `${queue.avgDelay5m.toFixed(2)}m`,
          (workerTags[queue.id] || 0).toString()
        ]))
        .render();

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

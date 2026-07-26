import { Worker, NativeConnection } from "@temporalio/worker";
import * as activities from "./activities";

async function run() {
  const address = process.env.TEMPORAL_ADDRESS || "localhost:7233";
  console.log(`Connecting to Temporal at ${address}...`);

  const connection = await NativeConnection.connect({ address });
  const worker = await Worker.create({
    connection,
    workflowsPath: require.resolve("./workflow"),
    activities,
    taskQueue: "benchmark",
    maxConcurrentWorkflowTaskExecution: 10,
    maxConcurrentActivityTaskExecution: 10,
  });

  console.log("Temporal worker started on task queue: benchmark");
  await worker.run();
}

run().catch((err) => {
  console.error("Worker failed:", err);
  process.exit(1);
});

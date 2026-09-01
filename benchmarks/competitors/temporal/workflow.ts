import { proxyActivities } from "@temporalio/workflow";
import type * as activities from "./activities";

const { stepA, stepB, stepC } = proxyActivities<typeof activities>({
  startToCloseTimeout: "10s",
});

export async function benchmarkWorkflow(): Promise<{
  a: number;
  b: number;
  c: number;
}> {
  const a = await stepA();
  const b = await stepB();
  const c = await stepC();
  return { a, b, c };
}

import { Inngest } from "inngest";

export const inngest = new Inngest({ id: "benchmark" });

export const benchmarkFn = inngest.createFunction(
  { id: "benchmark-3step" },
  { event: "benchmark/run" },
  async ({ step }) => {
    const a = await step.run("step-a", async () => 1);
    const b = await step.run("step-b", async () => 2);
    const c = await step.run("step-c", async () => 3);
    return { a, b, c };
  },
);

import * as restate from "@restatedev/restate-sdk";

export const benchmarkWorkflow = restate.workflow({
  name: "benchmark",
  handlers: {
    run: async (
      ctx: restate.WorkflowContext,
    ): Promise<{ a: number; b: number; c: number }> => {
      const a = await ctx.run("step-a", async () => 1);
      const b = await ctx.run("step-b", async () => 2);
      const c = await ctx.run("step-c", async () => 3);
      return { a, b, c };
    },
  },
});

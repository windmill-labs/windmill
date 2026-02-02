import { describe, test, expect, beforeAll } from "bun:test";
import { runPromptAndCapture, wasSkillInvoked, wasToolUsed, validateTestFolder } from "./test-utils";

describe("Windmill Skill Invocation", () => {
  beforeAll(() => {
    if (!process.env.ANTHROPIC_API_KEY) {
      throw new Error("ANTHROPIC_API_KEY environment variable is required");
    }
    validateTestFolder();
  });

  describe("Flow Creation", () => {
    test("'Create a Windmill flow' should invoke write-flow skill", async () => {
      const result = await runPromptAndCapture(
        "Create a Windmill flow that fetches data from an API and transforms it. Use placeholder URLs.",
        undefined,
        3
      );

      console.log("Tools used:", result.toolsUsed.map(t => t.tool));
      console.log("Skills invoked:", result.skillsInvoked);

      expect(wasToolUsed(result, "Skill")).toBe(true);
      expect(wasSkillInvoked(result, "write-flow")).toBe(true);
    }, { timeout: 120000 });
  });

  describe("Python Script Creation", () => {
    test("'Write a Windmill Python script' should invoke write-script-python3 skill", async () => {
      const result = await runPromptAndCapture(
        "Write a Windmill Python script that fetches data from https://api.example.com/users",
        undefined,
        3
      );

      console.log("Tools used:", result.toolsUsed.map(t => t.tool));
      console.log("Skills invoked:", result.skillsInvoked);

      expect(wasToolUsed(result, "Skill")).toBe(true);
      expect(wasSkillInvoked(result, "write-script-python3")).toBe(true);
    }, { timeout: 120000 });
  });

  describe("Bun Script Creation", () => {
    test("'Write a Windmill Bun/TypeScript script' should invoke write-script-bun skill", async () => {
      const result = await runPromptAndCapture(
        "Write a Windmill Bun script that processes JSON data",
        undefined,
        3
      );

      console.log("Tools used:", result.toolsUsed.map(t => t.tool));
      console.log("Skills invoked:", result.skillsInvoked);

      expect(wasToolUsed(result, "Skill")).toBe(true);
      expect(wasSkillInvoked(result, "write-script-bun")).toBe(true);
    }, { timeout: 120000 });
  });

  describe("Schedule Configuration", () => {
    test("'Create a Windmill schedule' should invoke schedules skill", async () => {
      const result = await runPromptAndCapture(
        "Create a Windmill schedule that runs a script daily at midnight",
        undefined,
        3
      );

      console.log("Tools used:", result.toolsUsed.map(t => t.tool));
      console.log("Skills invoked:", result.skillsInvoked);

      expect(wasToolUsed(result, "Skill")).toBe(true);
      expect(wasSkillInvoked(result, "schedules")).toBe(true);
    }, { timeout: 120000 });
  });

  describe("Trigger Configuration", () => {
    test("'Set up a Windmill webhook trigger' should invoke triggers skill", async () => {
      const result = await runPromptAndCapture(
        "Set up a Windmill HTTP trigger for a flow at /api/webhook",
        undefined,
        3
      );

      console.log("Tools used:", result.toolsUsed.map(t => t.tool));
      console.log("Skills invoked:", result.skillsInvoked);

      expect(wasToolUsed(result, "Skill")).toBe(true);
      expect(wasSkillInvoked(result, "triggers")).toBe(true);
    }, { timeout: 120000 });
  });
});

import { describe, test, expect, beforeAll } from "bun:test";
import { runPromptAndCapture, wasSkillInvoked, wasToolUsed } from "./test-utils";

// Get the windmill repo root (parent of cli/test-skills)
const repoRoot = new URL("../../..", import.meta.url).pathname;

describe("Skill Invocation", () => {
  beforeAll(() => {
    if (!process.env.ANTHROPIC_API_KEY) {
      throw new Error("ANTHROPIC_API_KEY environment variable is required");
    }
  });

  test("/commit should invoke commit skill", async () => {
    const result = await runPromptAndCapture("/commit", repoRoot, 1);

    expect(wasToolUsed(result, "Skill")).toBe(true);
    expect(wasSkillInvoked(result, "commit")).toBe(true);
  }, { timeout: 60000 });

  test("/pr should invoke pr skill", async () => {
    const result = await runPromptAndCapture("/pr", repoRoot, 1);

    expect(wasToolUsed(result, "Skill")).toBe(true);
    expect(wasSkillInvoked(result, "pr")).toBe(true);
  }, { timeout: 60000 });

  test("natural language 'create a commit' should invoke commit skill", async () => {
    const result = await runPromptAndCapture(
      "create a commit for my changes",
      repoRoot,
      3
    );

    expect(wasSkillInvoked(result, "commit")).toBe(true);
  }, { timeout: 120000 });

  test("natural language 'open a PR' should invoke pr skill", async () => {
    const result = await runPromptAndCapture(
      "open a pull request for my changes",
      repoRoot,
      3
    );

    expect(wasSkillInvoked(result, "pr")).toBe(true);
  }, { timeout: 120000 });
});

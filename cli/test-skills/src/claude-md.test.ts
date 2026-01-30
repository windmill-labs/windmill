import { describe, test, expect, beforeAll } from "bun:test";
import { runPromptAndCapture, wasSkillInvoked, wasToolUsed, getToolInputs } from "./test-utils";

// Get the windmill repo root (parent of cli/test-skills)
const repoRoot = new URL("../../..", import.meta.url).pathname;

describe("CLAUDE.md Guidance", () => {
  beforeAll(() => {
    if (!process.env.ANTHROPIC_API_KEY) {
      throw new Error("ANTHROPIC_API_KEY environment variable is required");
    }
  });

  test("Rust backend questions should invoke rust-backend skill", async () => {
    const result = await runPromptAndCapture(
      "How should I structure a new API endpoint in the Rust backend?",
      repoRoot,
      2
    );

    expect(wasSkillInvoked(result, "rust-backend")).toBe(true);
  }, { timeout: 120000 });

  test("Rust code modification should invoke rust-backend skill", async () => {
    const result = await runPromptAndCapture(
      "I need to add a new field to the Script struct in the backend",
      repoRoot,
      2
    );

    expect(wasSkillInvoked(result, "rust-backend")).toBe(true);
  }, { timeout: 120000 });

  test("Code review request should invoke code-review skill", async () => {
    const result = await runPromptAndCapture(
      "Please review my pull request",
      repoRoot,
      2
    );

    expect(wasSkillInvoked(result, "code-review")).toBe(true);
  }, { timeout: 120000 });

  test("Backend file exploration should read files before suggesting changes", async () => {
    const result = await runPromptAndCapture(
      "What does the job queue look like in the backend?",
      repoRoot,
      5
    );

    // Should use Read or Grep tool to explore the codebase
    const usedExplorationTools = wasToolUsed(result, "Read") ||
                                  wasToolUsed(result, "Grep") ||
                                  wasToolUsed(result, "Glob");
    expect(usedExplorationTools).toBe(true);
  }, { timeout: 180000 });

  test("Database queries should reference the schema", async () => {
    const result = await runPromptAndCapture(
      "How is the v2_job table structured?",
      repoRoot,
      3
    );

    // The response should mention job-related fields from the schema
    const mentionsJobFields = result.output.toLowerCase().includes("workspace_id") ||
                              result.output.toLowerCase().includes("created_at") ||
                              result.output.toLowerCase().includes("job_kind");
    expect(mentionsJobFields).toBe(true);
  }, { timeout: 120000 });
});

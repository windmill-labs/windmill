import { cpSync, existsSync, mkdirSync, rmSync } from "fs";
import { join } from "path";
import {
  getToolInputs as getSharedToolInputs,
  getGeneratedSkillsSource as getSharedGeneratedSkillsSource,
  runPromptAndCapture as runSharedPromptAndCapture,
  wasSkillInvoked as wasSharedSkillInvoked,
  wasToolUsed as wasSharedToolUsed,
  type PromptRunResult,
  type ToolInvocation
} from "../../../ai_evals/adapters/cli/runtime";

export type TestResult = PromptRunResult;

/**
 * Get the test-skills directory path
 */
export function getTestSkillsDir(): string {
  return new URL("..", import.meta.url).pathname;
}

/**
 * Get the test-folder directory path (where user places .claude/skills)
 */
export function getTestFolder(): string {
  return join(getTestSkillsDir(), "test-folder");
}

/**
 * Get the generated skills directory from the repo root.
 */
export function getGeneratedSkillsSource(): string {
  return getSharedGeneratedSkillsSource();
}

/**
 * Ensure test-folder exists and mirrors the repo's generated skills.
 */
export function validateTestFolder(): void {
  const testFolder = getTestFolder();
  const skillsFolder = join(testFolder, ".claude", "skills");
  const generatedSkillsSource = getGeneratedSkillsSource();

  if (!existsSync(generatedSkillsSource)) {
    throw new Error(
      `Generated skills directory not found at: ${generatedSkillsSource}\n` +
      `Run system prompt generation first so cli/test-skills can mirror the current repo skill bundle.`
    );
  }

  mkdirSync(join(testFolder, ".claude"), { recursive: true });
  rmSync(skillsFolder, { recursive: true, force: true });
  cpSync(generatedSkillsSource, skillsFolder, { recursive: true });
}

/**
 * Runs a prompt through the Claude Agent SDK and captures tool invocations
 * Uses test-folder as cwd where user-provided skills are located
 */
export async function runPromptAndCapture(
  prompt: string,
  cwd?: string,
  maxTurns: number = 3
): Promise<TestResult> {
  return runSharedPromptAndCapture(prompt, cwd ?? getTestFolder(), maxTurns);
}

/**
 * Helper to check if a specific tool was used
 */
export function wasToolUsed(result: TestResult, toolName: string): boolean {
  return wasSharedToolUsed(result, toolName);
}

/**
 * Helper to check if a specific skill was invoked
 */
export function wasSkillInvoked(result: TestResult, skillName: string): boolean {
  return wasSharedSkillInvoked(result, skillName);
}

/**
 * Helper to get all tool inputs for a specific tool
 */
export function getToolInputs(result: TestResult, toolName: string): Record<string, unknown>[] {
  return getSharedToolInputs(result, toolName);
}

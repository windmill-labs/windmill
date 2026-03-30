import { query, type Options } from "@anthropic-ai/claude-agent-sdk";
import { cpSync, existsSync, mkdirSync, rmSync } from "fs";
import { join } from "path";

export interface ToolInvocation {
  tool: string;
  input: Record<string, unknown>;
  timestamp: number;
}

export interface TestResult {
  toolsUsed: ToolInvocation[];
  skillsInvoked: string[];
  output: string;
}

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
  return join(getTestSkillsDir(), "..", "..", "system_prompts", "auto-generated", "skills");
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
  const workingDir = cwd ?? getTestFolder();
  const toolsUsed: ToolInvocation[] = [];
  const skillsInvoked: string[] = [];
  let output = "";

  const options: Options = {
    cwd: workingDir,
    model: "haiku",
    maxTurns,
    settingSources: ["project"],  // Required to load Skills from filesystem
    allowedTools: ["Skill", "Read", "Glob", "Grep", "Bash", "Write", "Edit"],
  };

  for await (const message of query({ prompt, options })) {
    if (message.type === "assistant") {
      // The assistant message has a BetaMessage which contains content blocks
      const content = message.message?.content;
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === "tool_use") {
            const toolInvocation: ToolInvocation = {
              tool: block.name,
              input: block.input as Record<string, unknown>,
              timestamp: Date.now(),
            };
            toolsUsed.push(toolInvocation);

            // Check if this is a Skill tool invocation
            if (block.name === "Skill" && typeof block.input === "object" && block.input !== null) {
              const skillInput = block.input as { skill?: string };
              if (skillInput.skill) {
                skillsInvoked.push(skillInput.skill);
              }
            }
          } else if (block.type === "text") {
            output += block.text;
          }
        }
      }
    } else if (message.type === "result") {
      // Capture final result if available
      const resultMessage = message as { result?: string };
      if (typeof resultMessage.result === "string") {
        output += resultMessage.result;
      }
    }
  }

  return {
    toolsUsed,
    skillsInvoked,
    output,
  };
}

/**
 * Helper to check if a specific tool was used
 */
export function wasToolUsed(result: TestResult, toolName: string): boolean {
  return result.toolsUsed.some((t) => t.tool === toolName);
}

/**
 * Helper to check if a specific skill was invoked
 */
export function wasSkillInvoked(result: TestResult, skillName: string): boolean {
  return result.skillsInvoked.some((s) => s === skillName || s.includes(skillName));
}

/**
 * Helper to get all tool inputs for a specific tool
 */
export function getToolInputs(result: TestResult, toolName: string): Record<string, unknown>[] {
  return result.toolsUsed.filter((t) => t.tool === toolName).map((t) => t.input);
}

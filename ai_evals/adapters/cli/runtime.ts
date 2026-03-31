import { query, type Options } from "@anthropic-ai/claude-agent-sdk";
import { join } from "path";
import { fileURLToPath } from "url";

export interface ToolInvocation {
  tool: string;
  input: Record<string, unknown>;
  timestamp: number;
}

export interface PromptRunResult {
  toolsUsed: ToolInvocation[];
  skillsInvoked: string[];
  output: string;
  durationMs: number;
  assistantMessageCount: number;
}

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));
export const CLI_BENCHMARK_PROVIDER = "anthropic";
export const CLI_BENCHMARK_MODEL = "haiku";

export function getGeneratedSkillsSource(): string {
  return join(REPO_ROOT, "system_prompts", "auto-generated", "skills");
}

export async function runPromptAndCapture(
  prompt: string,
  cwd: string,
  maxTurns: number = 3
): Promise<PromptRunResult> {
  const toolsUsed: ToolInvocation[] = [];
  const skillsInvoked: string[] = [];
  let output = "";
  let assistantMessageCount = 0;
  const startedAt = Date.now();

  const options: Options = {
    cwd,
    model: CLI_BENCHMARK_MODEL,
    maxTurns,
    settingSources: ["project"],
    allowedTools: ["Skill", "Read", "Glob", "Grep", "Bash", "Write", "Edit"]
  };

  for await (const message of query({ prompt, options })) {
    if (message.type === "assistant") {
      assistantMessageCount += 1;
      const content = message.message?.content;
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === "tool_use") {
            toolsUsed.push({
              tool: block.name,
              input: block.input as Record<string, unknown>,
              timestamp: Date.now()
            });

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
    durationMs: Date.now() - startedAt,
    assistantMessageCount
  };
}

export function wasSkillInvoked(result: PromptRunResult, skillName: string): boolean {
  return result.skillsInvoked.some((skill) => skill === skillName || skill.includes(skillName));
}

export function wasToolUsed(result: PromptRunResult, toolName: string): boolean {
  return result.toolsUsed.some((tool) => tool.tool === toolName);
}

export function getToolInputs(
  result: PromptRunResult,
  toolName: string
): Record<string, unknown>[] {
  return result.toolsUsed
    .filter((tool) => tool.tool === toolName)
    .map((tool) => tool.input);
}

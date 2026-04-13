import { query, type Options } from "@anthropic-ai/claude-agent-sdk";
import { join } from "path";
import { fileURLToPath } from "url";
import { getCliEvalModel, resolveEvalModel, type CliEvalModelConfig } from "../../core/models";
import type { BenchmarkTokenUsage } from "../../core/types";

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
  tokenUsage: BenchmarkTokenUsage | null;
}

interface AnthropicUsageLike {
  input_tokens?: number | null;
  output_tokens?: number | null;
  cache_creation_input_tokens?: number | null;
  cache_read_input_tokens?: number | null;
}

interface AnthropicModelUsageLike {
  inputTokens?: number | null;
  outputTokens?: number | null;
  cacheCreationInputTokens?: number | null;
  cacheReadInputTokens?: number | null;
}

interface CliResultMessageLike {
  type?: string;
  usage?: AnthropicUsageLike | null;
  modelUsage?: Record<string, AnthropicModelUsageLike> | null;
}

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));
export const DEFAULT_CLI_EVAL_MODEL: CliEvalModelConfig = getCliEvalModel(resolveEvalModel("cli"));

export function getGeneratedSkillsSource(): string {
  return join(REPO_ROOT, "system_prompts", "auto-generated", "skills");
}

export function anthropicUsageToBenchmarkTokenUsage(
  usage: AnthropicUsageLike | null | undefined
): BenchmarkTokenUsage | null {
  if (!usage) {
    return null;
  }

  const prompt =
    (usage.input_tokens ?? 0) +
    (usage.cache_creation_input_tokens ?? 0) +
    (usage.cache_read_input_tokens ?? 0);
  const completion = usage.output_tokens ?? 0;

  return {
    prompt,
    completion,
    total: prompt + completion,
  };
}

export function extractCliResultTokenUsage(message: unknown): BenchmarkTokenUsage | null {
  if (!message || typeof message !== "object") {
    return null;
  }

  const resultMessage = message as CliResultMessageLike;
  if (resultMessage.type !== "result") {
    return null;
  }

  const usage = anthropicUsageToBenchmarkTokenUsage(resultMessage.usage);
  if (usage) {
    return usage;
  }

  if (!resultMessage.modelUsage || typeof resultMessage.modelUsage !== "object") {
    return null;
  }

  let prompt = 0;
  let completion = 0;
  let sawModelUsage = false;

  for (const modelUsage of Object.values(resultMessage.modelUsage)) {
    if (!modelUsage || typeof modelUsage !== "object") {
      continue;
    }

    prompt +=
      (modelUsage.inputTokens ?? 0) +
      (modelUsage.cacheCreationInputTokens ?? 0) +
      (modelUsage.cacheReadInputTokens ?? 0);
    completion += modelUsage.outputTokens ?? 0;
    sawModelUsage = true;
  }

  if (!sawModelUsage) {
    return null;
  }

  return {
    prompt,
    completion,
    total: prompt + completion,
  };
}

export async function runPromptAndCapture(
  prompt: string,
  cwd: string,
  maxTurns: number = 3,
  modelConfig: CliEvalModelConfig = DEFAULT_CLI_EVAL_MODEL
): Promise<PromptRunResult> {
  const toolsUsed: ToolInvocation[] = [];
  const skillsInvoked: string[] = [];
  let output = "";
  let assistantMessageCount = 0;
  let tokenUsage: BenchmarkTokenUsage | null = null;
  const startedAt = Date.now();

  const options: Options = {
    cwd,
    model: modelConfig.model,
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
      tokenUsage = extractCliResultTokenUsage(message) ?? tokenUsage;
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
    assistantMessageCount,
    tokenUsage,
  };
}

export function wasSkillInvoked(result: PromptRunResult, skillName: string): boolean {
  return result.skillsInvoked.some((skill) => skill === skillName || skill.includes(skillName));
}

export function wasToolUsed(result: PromptRunResult, toolName: string): boolean {
  return result.toolsUsed.some((tool) => tool.tool === toolName);
}

export function formatCliRunModelLabel(modelConfig: CliEvalModelConfig): string {
  return `${modelConfig.provider}:${modelConfig.model}`;
}

export function getToolInputs(
  result: PromptRunResult,
  toolName: string
): Record<string, unknown>[] {
  return result.toolsUsed
    .filter((tool) => tool.tool === toolName)
    .map((tool) => tool.input);
}

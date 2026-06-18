import { query, type Options } from "@anthropic-ai/claude-agent-sdk";
import { chmod, mkdir, readFile, writeFile } from "node:fs/promises";
import { delimiter, join } from "path";
import { fileURLToPath } from "url";
import { getCliEvalModel, resolveEvalModel, type CliEvalModelConfig } from "../../core/models";
import type {
  BenchmarkTokenUsage,
  CliToolInvocation,
  CliTrace,
  CliWmillInvocation,
} from "../../core/types";

export type ToolInvocation = CliToolInvocation;

export interface PromptRunResult {
  output: string;
  durationMs: number;
  tokenUsage: BenchmarkTokenUsage | null;
  trace: CliTrace;
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
const WMILL_STUB_DIR_NAME = ".wmill-benchmark-bin";
const WMILL_LOG_FILE_NAME = ".wmill-benchmark-wmill-invocations.log";
const WMILL_LOG_MARKER = "__WMILL_BENCHMARK__";
const NEGATED_COMMAND_PREFIX = /(?:^|\b)(?:do not|don't|dont|never|instead of)\s+(?:run|use)?\s*$/i;
const COMMAND_STOP_WORDS = new Set([
  "and",
  "before",
  "after",
  "then",
  "instead",
  "otherwise",
  "because",
  "so",
  "if",
  "when",
  "while",
  "once",
]);
const COMMAND_STOP_TOKENS = new Set(["-", "–", "—", "|"]);

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
  const bashCommands: string[] = [];
  let output = "";
  let assistantMessageCount = 0;
  let tokenUsage: BenchmarkTokenUsage | null = null;
  const startedAt = Date.now();
  const stubBinDir = join(cwd, WMILL_STUB_DIR_NAME);
  const wmillLogPath = join(cwd, WMILL_LOG_FILE_NAME);

  const options: Options = {
    cwd,
    model: modelConfig.model,
    maxTurns,
    settingSources: ["project"],
    allowedTools: ["Skill", "Read", "Glob", "Grep", "Bash", "Write", "Edit"],
    env: {
      ...getQueryEnv(),
      PATH: process.env.PATH ? `${stubBinDir}${delimiter}${process.env.PATH}` : stubBinDir,
      WMILL_BENCHMARK_LOG_PATH: wmillLogPath,
    },
  };

  await installWmillStub(stubBinDir);

  for await (const message of query({ prompt, options })) {
    if (message.type === "assistant") {
      assistantMessageCount += 1;
      const content = message.message?.content;
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === "tool_use") {
            const input = normalizeToolInput(block.input);
            toolsUsed.push({
              tool: block.name,
              input,
              timestamp: Date.now()
            });

            if (block.name === "Skill") {
              const skillInput = input as { skill?: string };
              if (skillInput.skill) {
                pushUnique(skillsInvoked, skillInput.skill);
              }
            }

            if (block.name === "Bash") {
              for (const command of extractBashCommands(input)) {
                pushUnique(bashCommands, command);
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

  const proposedCommands = extractProposedWmillCommands(output);
  const wmillInvocations = await readWmillInvocationLog(wmillLogPath);

  return {
    output,
    durationMs: Date.now() - startedAt,
    tokenUsage,
    trace: {
      toolsUsed,
      skillsInvoked,
      assistantMessageCount,
      bashCommands,
      proposedCommands,
      executedWmillCommands: wmillInvocations.map(formatExecutedWmillCommand),
      wmillInvocations,
      firstMutationToolIndex: getFirstMutationToolIndex(toolsUsed),
    },
  };
}

export function wasSkillInvoked(result: PromptRunResult, skillName: string): boolean {
  return result.trace.skillsInvoked.some((skill) => skill === skillName);
}

export function wasToolUsed(result: PromptRunResult, toolName: string): boolean {
  return result.trace.toolsUsed.some((tool) => tool.tool === toolName);
}

export function formatCliRunModelLabel(modelConfig: CliEvalModelConfig): string {
  return `${modelConfig.provider}:${modelConfig.model}`;
}

export function getToolInputs(
  result: PromptRunResult,
  toolName: string
): Record<string, unknown>[] {
  return result.trace.toolsUsed
    .filter((tool) => tool.tool === toolName)
    .map((tool) => tool.input);
}

export function extractProposedWmillCommands(output: string): string[] {
  const commands: string[] = [];

  for (const line of output.split(/\r?\n/)) {
    for (const command of extractInlineBacktickCommands(line)) {
      pushUnique(commands, command);
    }

    for (const command of extractInlineProseCommands(line.replace(/^\s*(?:[-*]|\d+\.)\s*/, ""))) {
      pushUnique(commands, command);
    }
  }

  return commands;
}

export function parseWmillInvocationLog(raw: string): CliWmillInvocation[] {
  const entries: CliWmillInvocation[] = [];
  const lines = raw.split(/\r?\n/);

  for (let index = 0; index < lines.length; index += 1) {
    if (lines[index] !== WMILL_LOG_MARKER) {
      continue;
    }

    const timestamp = lines[index + 1] ?? "";
    const cwd = lines[index + 2] ?? "";
    const argCount = Number.parseInt(lines[index + 3] ?? "", 10);
    if (!Number.isFinite(argCount) || argCount < 0) {
      continue;
    }

    const start = index + 4;
    const argv = lines.slice(start, start + argCount);
    entries.push({ argv, cwd, timestamp });
    index = start + argCount - 1;
  }

  return entries;
}

async function installWmillStub(binDir: string): Promise<void> {
  await mkdir(binDir, { recursive: true });

  const stubPath = join(binDir, "wmill");
  const script = `#!/usr/bin/env bash
set -euo pipefail
{
  printf '${WMILL_LOG_MARKER}\\n'
  date -u +"%Y-%m-%dT%H:%M:%SZ"
  printf '%s\\n' "$PWD"
  printf '%s\\n' "$#"
  printf '%s\\n' "$@"
} >> "\${WMILL_BENCHMARK_LOG_PATH:?}"
printf 'wmill benchmark stub: do not execute Windmill CLI commands during ai_evals; describe them in the final response instead.\\n' >&2
exit 97
`;

  await writeFile(stubPath, script, "utf8");
  await chmod(stubPath, 0o755);
}

async function readWmillInvocationLog(logPath: string): Promise<CliWmillInvocation[]> {
  const raw = await readFile(logPath, "utf8").catch(() => null);
  if (!raw) {
    return [];
  }
  return parseWmillInvocationLog(raw);
}

function getQueryEnv(): Record<string, string> {
  return Object.fromEntries(
    Object.entries(process.env).flatMap(([key, value]) =>
      typeof value === "string" ? [[key, value]] : []
    )
  );
}

function normalizeToolInput(input: unknown): Record<string, unknown> {
  if (input && typeof input === "object" && !Array.isArray(input)) {
    return input as Record<string, unknown>;
  }

  if (typeof input === "string") {
    return { raw: input };
  }

  return {};
}

function extractBashCommands(input: Record<string, unknown>): string[] {
  const commands: string[] = [];

  for (const key of ["command", "cmd", "script", "raw"]) {
    const value = input[key];
    if (typeof value === "string") {
      for (const line of value.split(/\r?\n/)) {
        const command = normalizeCommandCandidate(line);
        if (command) {
          pushUnique(commands, command);
        }
      }
    }
  }

  return commands;
}

function extractInlineBacktickCommands(line: string): string[] {
  const commands: string[] = [];
  const regex = /`(wmill [^`\n]+)`/g;
  let match: RegExpExecArray | null = null;

  while ((match = regex.exec(line)) !== null) {
    if (hasNegatedCommandPrefix(line.slice(0, match.index))) {
      continue;
    }

    const command = normalizeCommandCandidate(match[1]);
    if (command) {
      pushUnique(commands, command);
    }
  }

  return commands;
}

function extractInlineProseCommands(line: string): string[] {
  const commands: string[] = [];
  let searchFrom = 0;

  while (true) {
    const inlineIndex = line.toLowerCase().indexOf("wmill ", searchFrom);
    if (inlineIndex === -1) {
      return commands;
    }

    if (!hasNegatedCommandPrefix(line.slice(0, inlineIndex))) {
      const command = extractInlineProseCommandAt(line, inlineIndex);
      if (command) {
        pushUnique(commands, command);
      }
    }

    searchFrom = inlineIndex + "wmill ".length;
  }
}

function extractInlineProseCommandAt(line: string, startIndex: number): string | null {
  const tokens = ["wmill"];
  let cursor = startIndex + "wmill".length;

  while (cursor < line.length) {
    while (cursor < line.length && /\s/.test(line[cursor]!)) {
      cursor += 1;
    }

    if (cursor >= line.length) {
      break;
    }

    const current = line[cursor]!;
    if ("`.,;:()[]{}".includes(current)) {
      break;
    }

    const token = readCommandToken(line, cursor);
    if (!token) {
      break;
    }

    if (COMMAND_STOP_WORDS.has(token.value.toLowerCase())) {
      break;
    }

    if (COMMAND_STOP_TOKENS.has(token.value)) {
      break;
    }

    tokens.push(token.value);
    cursor = token.nextIndex;
  }

  if (tokens.length <= 1) {
    return null;
  }

  return normalizeCommandCandidate(tokens.join(" "));
}

function readCommandToken(
  line: string,
  startIndex: number
): { value: string; nextIndex: number } | null {
  const firstChar = line[startIndex]!;

  if (firstChar === `"` || firstChar === `'`) {
    const endIndex = line.indexOf(firstChar, startIndex + 1);
    const nextIndex = endIndex === -1 ? line.length : endIndex + 1;
    return {
      value: line.slice(startIndex, nextIndex),
      nextIndex,
    };
  }

  if (firstChar === "<") {
    const endIndex = line.indexOf(">", startIndex + 1);
    const nextIndex = endIndex === -1 ? line.length : endIndex + 1;
    return {
      value: line.slice(startIndex, nextIndex),
      nextIndex,
    };
  }

  let endIndex = startIndex;
  while (endIndex < line.length && !/[\s`.,;:()[\]{}#]/.test(line[endIndex]!)) {
    endIndex += 1;
  }

  if (endIndex === startIndex) {
    return null;
  }

  return {
    value: line.slice(startIndex, endIndex),
    nextIndex: endIndex,
  };
}

function hasNegatedCommandPrefix(prefix: string): boolean {
  const normalizedPrefix = prefix
    .toLowerCase()
    .replace(/[`"'“”‘’]/g, " ")
    .replace(/\s+/g, " ")
    .trimEnd();

  return NEGATED_COMMAND_PREFIX.test(normalizedPrefix);
}

function normalizeCommandCandidate(value: string): string | null {
  const trimmed = value.trim().replace(/^`|`$/g, "");
  if (!trimmed) {
    return null;
  }

  const normalized = trimmed
    .replace(/\s+/g, " ")
    .replace(/[`.;:,]+$/g, "")
    .trim();

  return normalized.length > 0 ? normalized : null;
}

function formatExecutedWmillCommand(entry: CliWmillInvocation): string {
  return ["wmill", ...entry.argv].join(" ").trim();
}

function getFirstMutationToolIndex(toolsUsed: ToolInvocation[]): number | null {
  for (const [index, tool] of toolsUsed.entries()) {
    if (tool.tool === "Write" || tool.tool === "Edit") {
      return index;
    }

    if (tool.tool === "Bash" && extractBashCommands(tool.input).some(isLikelyMutatingBashCommand)) {
      return index;
    }
  }

  return null;
}

function isLikelyMutatingBashCommand(command: string): boolean {
  return (
    /\b(?:mkdir|touch|rm|mv|cp|install|tee)\b/.test(command) ||
    /\b(?:cat|echo|printf)\b.*(?:>|>>|\|\s*tee\b)/.test(command) ||
    /\bsed\s+-i\b/.test(command) ||
    /\bperl\s+-pi\b/.test(command) ||
    /\bwmill\b/.test(command)
  );
}

function pushUnique(values: string[], value: string): void {
  if (!values.includes(value)) {
    values.push(value);
  }
}

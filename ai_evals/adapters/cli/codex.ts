import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import type { BenchmarkTokenUsage, CliToolInvocation } from "../../core/types";

export interface AgentRunResult {
  output: string;
  toolsUsed: CliToolInvocation[];
  assistantMessageCount: number;
  tokenUsage: BenchmarkTokenUsage | null;
  finalContextTokens: number | null;
}

interface CodexItem {
  type?: string;
  text?: string;
  command?: string;
  changes?: { path?: string; kind?: string }[];
  [key: string]: unknown;
}

interface CodexEvent {
  type?: string;
  item?: CodexItem;
  usage?: { input_tokens?: number; output_tokens?: number };
  message?: string;
  error?: { message?: string };
}

const SKILL_FILE_PATTERN = /\.agents\/skills\/([^/\s'"`]+)\/SKILL\.md/g;

// Drives `codex exec` and maps its events onto the Claude Code tool vocabulary
// the validators read: shell commands become `Bash`, patches `Edit`, and
// reading `.agents/skills/<name>/SKILL.md` (how Codex loads a skill) becomes a
// `Skill` call recorded just before that command.
export async function runCodex(
  prompt: string,
  cwd: string,
  maxTurns: number,
  model: string,
  env: Record<string, string>
): Promise<AgentRunResult> {
  // A fresh HOME keeps the user's ~/.codex config and ~/.agents/skills out of the run.
  const home = await mkdtemp(join(tmpdir(), "wmill-cli-benchmark-codex-home-"));
  const result: AgentRunResult = {
    output: "",
    toolsUsed: [],
    assistantMessageCount: 0,
    tokenUsage: null,
    finalContextTokens: null,
  };
  const messages: string[] = [];
  let failure: string | null = null;
  let stderr = "";

  try {
    const child = spawn(
      "codex",
      [
        "exec",
        "--json",
        "--ephemeral",
        "--skip-git-repo-check",
        "--dangerously-bypass-approvals-and-sandbox",
        "--model",
        model,
        "--cd",
        cwd,
        "-",
      ],
      {
        cwd,
        env: {
          ...env,
          HOME: home,
          CODEX_HOME: join(home, ".codex"),
          CODEX_API_KEY: env.OPENAI_API_KEY ?? "",
        },
        stdio: ["pipe", "pipe", "pipe"],
      }
    );
    child.stdin.end(prompt);
    child.stderr.on("data", (chunk) => {
      stderr = (stderr + chunk).slice(-4000);
    });
    const exited = new Promise<number | null>((resolve, reject) => {
      child.on("error", reject);
      child.on("close", resolve);
    });

    // Codex has no turn cap. A turn starts when the agent issues tool calls with
    // none in flight, so parallel calls count once, as they do for Claude Code.
    let turns = 0;
    let inFlight = 0;
    let capped = false;

    for await (const line of createInterface({ input: child.stdout })) {
      const event = parseEvent(line);
      if (!event) {
        continue;
      }
      const item = event.item;

      if (event.type === "item.started" && item && isToolItem(item)) {
        if (inFlight === 0) {
          turns += 1;
        }
        inFlight += 1;
        if (turns > maxTurns && !capped) {
          capped = true;
          child.kill("SIGTERM");
        }
      } else if (event.type === "item.completed" && item) {
        if (item.type === "agent_message") {
          result.assistantMessageCount += 1;
          if (typeof item.text === "string") {
            messages.push(item.text);
          }
        } else if (isToolItem(item)) {
          inFlight = Math.max(0, inFlight - 1);
          if (!capped) {
            result.toolsUsed.push(...toToolInvocations(item));
          }
        }
      } else if (event.type === "turn.completed" && event.usage) {
        const prompt = event.usage.input_tokens ?? 0;
        const completion = event.usage.output_tokens ?? 0;
        result.tokenUsage = { prompt, completion, total: prompt + completion };
      } else if (event.type === "turn.failed" || event.type === "error") {
        failure = event.error?.message ?? event.message ?? JSON.stringify(event);
      }
    }

    const code = await exited;
    if (failure) {
      throw new Error(`codex: ${failure}`);
    }
    if (code !== 0 && !capped) {
      throw new Error(`codex exited with ${code}: ${stderr.trim()}`);
    }
  } finally {
    await rm(home, { recursive: true, force: true });
  }

  result.output = messages.join("\n");
  return result;
}

export function toToolInvocations(item: CodexItem): CliToolInvocation[] {
  const timestamp = Date.now();

  if (item.type === "command_execution" && typeof item.command === "string") {
    const command = unwrapShellCommand(item.command);
    const skills = [...new Set([...command.matchAll(SKILL_FILE_PATTERN)].map((match) => match[1]!))];
    return [
      ...skills.map((skill) => ({ tool: "Skill", input: { skill }, timestamp })),
      { tool: "Bash", input: { command }, timestamp },
    ];
  }

  if (item.type === "file_change") {
    return [{ tool: "Edit", input: { changes: item.changes ?? [] }, timestamp }];
  }

  const { id: _id, type, status: _status, ...input } = item;
  return [{ tool: String(type), input, timestamp }];
}

// Codex reports commands as run through the user's login shell, e.g.
// `/usr/bin/zsh -lc 'cat SKILL.md'`.
export function unwrapShellCommand(command: string): string {
  const match = command.match(/^\S*\/(?:ba|z)?sh -l?c (['"])([\s\S]*)\1$/);
  if (!match) {
    return command;
  }
  const body = match[2]!;
  return match[1] === "'" ? body.replaceAll(`'\\''`, "'") : body.replace(/\\(["\\$`])/g, "$1");
}

function isToolItem(item: CodexItem): boolean {
  return !["agent_message", "reasoning", "todo_list", "error"].includes(item.type ?? "");
}

function parseEvent(line: string): CodexEvent | null {
  try {
    const value = JSON.parse(line);
    return value && typeof value === "object" ? (value as CodexEvent) : null;
  } catch {
    return null;
  }
}

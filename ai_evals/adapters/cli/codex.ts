import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import type { BenchmarkTokenUsage, CliToolInvocation } from "../../core/types";
import { isLikelyMutatingBashCommand } from "./mutation";

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

const SKILL_FILE_PATTERN = /\.agents\/skills\/([^/\s'"`*?[]+)\/SKILL\.md/g;

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
  let lastError: string | null = null;
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
          CODEX_HOME: home,
          CODEX_API_KEY: env.OPENAI_API_KEY ?? "",
        },
        stdio: ["pipe", "pipe", "pipe"],
      }
    );
    // Without this, a missing `codex` binary leaves the stdout loop waiting forever.
    await new Promise<void>((resolve, reject) => {
      child.once("spawn", resolve);
      child.once("error", reject);
    });
    child.stdin.on("error", () => {});
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
    // Claude Code's cap includes the final reply, so the maxTurns-th tool round is
    // already over it.
    // ponytail: parallel calls that finish before the next one starts count as
    // separate turns; count model calls if Codex ever reports them.
    let turns = 0;
    let inFlight = 0;
    let capped = false;
    // Parallel calls complete out of order; the trace keeps the order they started in.
    const slots = new Map<string, number>();
    const ordered: CliToolInvocation[][] = [];
    const slotFor = (item: CodexItem) => {
      const id = typeof item.id === "string" ? item.id : undefined;
      if (id !== undefined && slots.has(id)) {
        return slots.get(id)!;
      }
      ordered.push([]);
      if (id !== undefined) {
        slots.set(id, ordered.length - 1);
      }
      return ordered.length - 1;
    };

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
        slotFor(item);
        if (turns >= maxTurns && !capped) {
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
          ordered[slotFor(item)] = toToolInvocations(item);
        }
      } else if (event.type === "turn.completed" && event.usage) {
        const prompt = event.usage.input_tokens ?? 0;
        const completion = event.usage.output_tokens ?? 0;
        result.tokenUsage = { prompt, completion, total: prompt + completion };
      } else if (event.type === "turn.failed") {
        failure = event.error?.message ?? JSON.stringify(event);
      } else if (event.type === "error") {
        // Also sent for retries Codex recovers from ("Reconnecting... 1/5").
        lastError = event.message ?? JSON.stringify(event);
      }
    }

    result.toolsUsed = ordered.flat();
    const code = await exited;
    if (failure) {
      throw new Error(`codex: ${failure}`);
    }
    // Fail the attempt, as Claude Code does at its cap.
    if (capped) {
      throw new Error(`codex: reached maximum number of turns (${maxTurns})`);
    }
    if (code !== 0) {
      throw new Error(`codex exited with ${code}: ${lastError ?? stderr.trim()}`);
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
    // A skill read counts as preceding the command only when nothing before it in
    // the command mutates; otherwise `printf x > f && cat SKILL.md` would pass a
    // skill-before-first-mutation check.
    const before: string[] = [];
    const after: string[] = [];
    for (const match of command.matchAll(SKILL_FILE_PATTERN)) {
      const skill = match[1]!;
      if (before.includes(skill) || after.includes(skill)) {
        continue;
      }
      (isLikelyMutatingBashCommand(command.slice(0, match.index)) ? after : before).push(skill);
    }
    const skillCall = (skill: string) => ({ tool: "Skill", input: { skill }, timestamp });
    return [
      ...before.map(skillCall),
      { tool: "Bash", input: { command }, timestamp },
      ...after.map(skillCall),
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
  const match = command.match(/^(?:\S*\/)?(?:ba|z)?sh -l?c (['"])([\s\S]*)\1$/);
  if (!match) {
    return command;
  }
  const body = match[2]!;
  return match[1] === "'"
    ? body.replaceAll(`'\\''`, "'").replaceAll(`'"'"'`, "'")
    : body.replace(/\\(["\\$`])/g, "$1");
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

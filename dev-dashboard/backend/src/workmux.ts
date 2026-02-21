import { $ } from "bun";

export interface Worktree {
  branch: string;
  agent: string;
  mux: string;
  unmerged: string;
  path: string;
}

export interface WorktreeStatus {
  worktree: string;
  status: string;
  elapsed: string;
  title: string;
}

function parseTable<T>(output: string, mapper: (cols: string[]) => T): T[] {
  const lines = output.trim().split("\n").filter(Boolean);
  if (lines.length < 2) return [];

  const headerLine = lines[0];

  // Find column positions based on header spacing
  const colStarts: number[] = [];
  let inSpace = true;
  for (let i = 0; i < headerLine.length; i++) {
    if (headerLine[i] !== " " && inSpace) {
      colStarts.push(i);
      inSpace = false;
    } else if (headerLine[i] === " " && !inSpace) {
      inSpace = true;
    }
  }

  return lines.slice(1).map(line => {
    const cols = colStarts.map((start, idx) => {
      const end = idx + 1 < colStarts.length ? colStarts[idx + 1] : line.length;
      return line.slice(start, end).trim();
    });
    return mapper(cols);
  });
}

export async function listWorktrees(): Promise<Worktree[]> {
  const result = await $`workmux list`.text();
  return parseTable(result, (cols) => ({
    branch: cols[0] ?? "",
    agent: cols[1] ?? "",
    mux: cols[2] ?? "",
    unmerged: cols[3] ?? "",
    path: cols[4] ?? "",
  }));
}

export async function getStatus(): Promise<WorktreeStatus[]> {
  const result = await $`workmux status`.text();
  return parseTable(result, (cols) => ({
    worktree: cols[0] ?? "",
    status: cols[1] ?? "",
    elapsed: cols[2] ?? "",
    title: cols[3] ?? "",
  }));
}

async function runChecked(args: string[]): Promise<string> {
  const proc = Bun.spawn(args, { stdout: "pipe", stderr: "pipe" });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const exitCode = await proc.exited;

  if (exitCode !== 0) {
    throw new Error(`${args.join(" ")} failed: ${stderr || stdout}`);
  }
  return stdout.trim();
}

export type Profile = "full" | "agent-only" | "agent-yolo";

function readEnvLocal(wtDir: string): Record<string, string> {
  try {
    const content = Bun.spawnSync(["cat", `${wtDir}/.env.local`], { stdout: "pipe" });
    const text = new TextDecoder().decode(content.stdout).trim();
    const env: Record<string, string> = {};
    for (const line of text.split("\n")) {
      const match = line.match(/^(\w+)=(.*)$/);
      if (match) env[match[1]] = match[2];
    }
    return env;
  } catch {
    return {};
  }
}

function buildSystemPrompt(profile: Profile, env: Record<string, string>): string {
  const backendPort = env.BACKEND_PORT || "8000";
  const frontendPort = env.FRONTEND_PORT || "3000";
  const lines: string[] = [];

  if (profile === "agent-yolo") {
    lines.push("You are running inside a sandboxed container with full permissions.");
  }

  lines.push(`This worktree is configured with the following ports:`);
  lines.push(`- Backend: port ${backendPort}. Start with: cd backend && DATABASE_URL=postgres://postgres:changeme@localhost:5432/windmill cargo run -- --port ${backendPort}`);
  lines.push(`- Frontend: port ${frontendPort}. Start with: cd frontend && REMOTE=http://localhost:${backendPort} npm run dev -- --port ${frontendPort} --host 0.0.0.0`);

  return lines.join(" ");
}

function buildClaudeCmd(profile: Profile, env: Record<string, string>): string {
  const prompt = buildSystemPrompt(profile, env);

  if (profile === "agent-yolo") {
    // Double-escape: outer single quotes for the host shell,
    // inner double quotes to survive workmux sandbox's sh -c
    const innerEscaped = prompt.replace(/["\\$`]/g, "\\$&");
    return `workmux sandbox agent -- claude --dangerously-skip-permissions --append-system-prompt '"${innerEscaped}"'`;
  }
  const escapedPrompt = prompt.replace(/'/g, "'\\''");
  return `claude --append-system-prompt '${escapedPrompt}'`;
}

export async function addWorktree(
  branch: string,
  opts?: { prompt?: string; profile?: Profile }
): Promise<string> {
  const profile = opts?.profile ?? "full";
  const args: string[] = ["workmux", "add", "-b"]; // -b = background (don't switch tmux)

  // Skip default pane commands for non-full profiles
  if (profile !== "full") {
    args.push("-C"); // --no-pane-cmds
  }

  // Enable sandbox for yolo profile (safe to skip permissions inside container)
  if (profile === "agent-yolo") {
    args.push("-S"); // --sandbox
  }

  if (opts?.prompt) args.push("-p", opts.prompt);
  args.push(branch);

  const result = await runChecked(args);

  // For non-full profiles, kill extra panes and send commands
  if (profile !== "full") {
    const windowTarget = `wm-${branch}`;
    // Kill extra panes (highest index first to avoid shifting)
    const paneCountResult = Bun.spawnSync(
      ["tmux", "list-panes", "-t", windowTarget, "-F", "#{pane_index}"],
      { stdout: "pipe" }
    );
    const paneIds = new TextDecoder().decode(paneCountResult.stdout).trim().split("\n");
    // Kill all panes except pane 0
    for (let i = paneIds.length - 1; i >= 1; i--) {
      Bun.spawnSync(["tmux", "kill-pane", "-t", `${windowTarget}.${paneIds[i]}`]);
    }
    // Get the worktree directory from pane 0
    const cwdResult = Bun.spawnSync(
      ["tmux", "display-message", "-t", `${windowTarget}.0`, "-p", "#{pane_current_path}"],
      { stdout: "pipe" }
    );
    const wtDir = new TextDecoder().decode(cwdResult.stdout).trim();
    // Build and send claude command with environment-aware system prompt
    const env = readEnvLocal(wtDir);
    const claudeCmd = buildClaudeCmd(profile, env);
    console.log(`[workmux] sending command to ${windowTarget}.0:\n${claudeCmd}`);
    Bun.spawnSync(["tmux", "send-keys", "-t", `${windowTarget}.0`, claudeCmd, "Enter"]);
    // Open a shell pane on the right (1/3 width) in the worktree dir
    Bun.spawnSync(["tmux", "split-window", "-h", "-t", `${windowTarget}.0`, "-l", "33%", "-c", wtDir]);
    // Keep focus on the agent pane (left)
    Bun.spawnSync(["tmux", "select-pane", "-t", `${windowTarget}.0`]);
  }

  return result;
}

export async function removeWorktree(name: string): Promise<string> {
  return runChecked(["workmux", "rm", "--force", name]);
}

export async function openWorktree(name: string): Promise<string> {
  return runChecked(["workmux", "open", name]);
}

export async function closeWorktree(name: string): Promise<string> {
  return runChecked(["workmux", "close", name]);
}

export async function sendPrompt(name: string, prompt: string): Promise<string> {
  return runChecked(["workmux", "send", name, prompt]);
}

export async function getTmuxSession(): Promise<string> {
  try {
    const result = await $`tmux list-windows -a -F "#{session_name}:#{window_name}"`.text();
    for (const line of result.trim().split("\n")) {
      const [session, window] = line.split(":");
      if (window?.startsWith("wm-")) {
        return session!;
      }
    }
  } catch {
    // No tmux server running
  }
  return "0";
}

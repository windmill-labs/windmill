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

const PROFILE_PANE_CMDS: Record<Profile, string[]> = {
  "full": [], // Use default workmux pane commands from .workmux.yaml
  "agent-only": ["claude"], // Only start agent in pane 0
  "agent-yolo": ["workmux sandbox agent -- claude --dangerously-skip-permissions"],
};

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
    // Send commands to remaining pane
    const cmds = PROFILE_PANE_CMDS[profile];
    for (const cmd of cmds) {
      Bun.spawnSync(["tmux", "send-keys", "-t", `${windowTarget}.0`, cmd, "Enter"]);
    }
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

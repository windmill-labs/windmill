import { $ } from "bun";
import { startForwarding, stopForwarding } from "./socat";
import { readEnvLocal } from "./env";

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
    const msg = `${args.join(" ")} failed (exit ${exitCode}): ${stderr || stdout}`;
    console.error(`[workmux:exec] ${msg}`);
    throw new Error(msg);
  }
  return stdout.trim();
}

export type Profile = "full" | "agent-yolo";
export type Agent = "claude" | "codex";

export { readEnvLocal } from "./env";

function buildSandboxSystemPrompt(env: Record<string, string>): string {
  const backendPort = env.BACKEND_PORT || "8000";
  const frontendPort = env.FRONTEND_PORT || "3000";
  const hasR2 = !!(process.env.R2_ENDPOINT && process.env.R2_BUCKET && process.env.R2_PUBLIC_URL);
  console.log(`[workmux:buildSandboxSystemPrompt] hasR2=${hasR2}`);
  const lines: string[] = [
    "You are running inside a sandboxed container with full permissions.",
    `This worktree is configured with the following ports:`,
    `- Backend: port ${backendPort}. Start with: cd backend && PORT=${backendPort} DATABASE_URL=postgres://postgres:changeme@localhost:5432/windmill cargo watch -x run`,
    `- Frontend: port ${frontendPort}. Start with: cd frontend && REMOTE=http://localhost:${backendPort} npm run dev -- --port ${frontendPort} --host 0.0.0.0`,
  ];
  if (hasR2) {
    lines.push(
      `--- Screenshots ---`,
      `You can take screenshots of the frontend UI and upload them to R2 for use in PR descriptions.`,
      `1) Take a screenshot: PLAYWRIGHT_BROWSERS_PATH=/opt/playwright-browsers bunx playwright screenshot --browser chromium http://localhost:${frontendPort}/path/to/page /tmp/screenshot.png`,
      `2) Upload to R2: aws s3 cp /tmp/screenshot.png "s3://$R2_BUCKET/$(git rev-parse --abbrev-ref HEAD)/screenshot.png" --endpoint-url "$R2_ENDPOINT"`,
      `3) The public URL will be: $R2_PUBLIC_URL/<branch>/screenshot.png`,
      `4) Include screenshots in PR descriptions as markdown images: ![description]($R2_PUBLIC_URL/<branch>/screenshot.png)`,
    );
  }
  return lines.join(" ");
}

/** Env vars to forward into the sandbox container (via workmux env_passthrough). */
const SANDBOX_ENV_PASSTHROUGH = [
  "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY",
  "R2_ENDPOINT",
  "R2_BUCKET",
  "R2_PUBLIC_URL",
];

/** Build an inline env prefix (e.g. "KEY=val KEY2=val2 ") from process.env. */
function buildEnvPrefix(): string {
  const parts: string[] = [];
  for (const key of SANDBOX_ENV_PASSTHROUGH) {
    const val = process.env[key];
    if (val) {
      // Shell-escape the value (single quotes, escaping inner single quotes)
      const escaped = val.replace(/'/g, "'\\''");
      parts.push(`${key}='${escaped}'`);
    }
  }
  return parts.length > 0 ? parts.join(" ") + " " : "";
}

function buildSandboxAgentCmd(env: Record<string, string>, agent: Agent): string {
  const prompt = buildSandboxSystemPrompt(env);
  const innerEscaped = prompt.replace(/["\\$`]/g, "\\$&");
  const envPrefix = buildEnvPrefix();

  if (agent === "codex") {
    return `${envPrefix}workmux sandbox agent -- codex --yolo -c '"developer_instructions=${innerEscaped}"'`;
  }
  return `${envPrefix}workmux sandbox agent -- claude --dangerously-skip-permissions --append-system-prompt '"${innerEscaped}"'`;
}

export async function addWorktree(
  branch: string,
  opts?: { prompt?: string; profile?: Profile; agent?: Agent }
): Promise<string> {
  const profile = opts?.profile ?? "full";
  const agent = opts?.agent ?? "claude";
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

  console.log(`[workmux:add] running: ${args.join(" ")}`);
  const result = await runChecked(args);
  console.log(`[workmux:add] result: ${result}`);

  const windowTarget = `wm-${branch}`;

  // Read worktree dir and log assigned ports
  const wtDirResult = Bun.spawnSync(
    ["tmux", "display-message", "-t", `${windowTarget}.0`, "-p", "#{pane_current_path}"],
    { stdout: "pipe" }
  );
  const wtDir = new TextDecoder().decode(wtDirResult.stdout).trim();
  const env = readEnvLocal(wtDir);
  console.log(`[workmux:add] branch=${branch} dir=${wtDir} ports: backend=${env.BACKEND_PORT || "8000"} frontend=${env.FRONTEND_PORT || "3000"}`);

  // Append profile to .env.local (worktree-env creates it, we just add to it)
  if (wtDir) {
    const envPath = `${wtDir}/.env.local`;
    const existing = await Bun.file(envPath).text().catch(() => "");
    if (!existing.includes("PROFILE=")) {
      await Bun.write(envPath, existing.trimEnd() + `\nPROFILE=${profile}\nAGENT=${agent}\n`);
    }
  }

  // For non-full profiles, kill extra panes and send commands
  if (profile !== "full") {
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
    // Build and send agent command for sandbox (env vars are inlined as a prefix)
    const agentCmd = buildSandboxAgentCmd(env, agent);
    console.log(`[workmux] sending command to ${windowTarget}.0:\n${agentCmd}`);
    Bun.spawnSync(["tmux", "send-keys", "-t", `${windowTarget}.0`, agentCmd, "Enter"]);
    // Open a shell pane on the right (1/3 width) in the worktree dir
    Bun.spawnSync(["tmux", "split-window", "-h", "-t", `${windowTarget}.0`, "-l", "33%", "-c", wtDir]);
    // Keep focus on the agent pane (left)
    Bun.spawnSync(["tmux", "select-pane", "-t", `${windowTarget}.0`]);

    // Start socat port forwarding for sandbox containers (non-blocking).
    // The container takes a few seconds to start after the tmux command is sent,
    // so we poll in the background rather than blocking the API response.
    if (profile === "agent-yolo" && wtDir) {
      (async () => {
        console.log(`[socat] waiting for container to start for ${branch}...`);
        for (let i = 1; i <= 15; i++) {
          await new Promise(r => setTimeout(r, 2000));
          if (await startForwarding(branch, wtDir)) return;
          console.log(`[socat] container not ready for ${branch}, retrying (${i}/15)...`);
        }
        console.error(`[socat] gave up waiting for container for ${branch} after 30s`);
      })();
    }
  }

  return result;
}

export async function removeWorktree(name: string): Promise<string> {
  console.log(`[workmux:rm] running: workmux rm --force ${name}`);
  stopForwarding(name);
  const result = await runChecked(["workmux", "rm", "--force", name]);
  console.log(`[workmux:rm] result: ${result}`);
  return result;
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

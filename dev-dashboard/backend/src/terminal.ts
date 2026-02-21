import { getTmuxSession } from "./workmux";

interface TerminalSession {
  proc: ReturnType<typeof Bun.spawn>;
  groupedSessionName: string;
  scrollback: string[];
  onData: ((data: string) => void) | null;
  onExit: ((exitCode: number) => void) | null;
}

const SESSION_PREFIX = "wm-dash-";
const MAX_SCROLLBACK = 5000;
const sessions = new Map<string, TerminalSession>();
let sessionCounter = 0;

function groupedName(): string {
  return `${SESSION_PREFIX}${++sessionCounter}`;
}

/** Kill any orphaned wm-dash-* tmux sessions left from previous server runs. */
export function cleanupStaleSessions(): void {
  try {
    const result = Bun.spawnSync(
      ["tmux", "list-sessions", "-F", "#{session_name}"],
      { stdout: "pipe", stderr: "pipe" }
    );
    if (result.exitCode !== 0) return;
    const lines = new TextDecoder().decode(result.stdout).trim().split("\n");
    for (const name of lines) {
      if (name.startsWith(SESSION_PREFIX)) {
        Bun.spawnSync(["tmux", "kill-session", "-t", name]);
      }
    }
  } catch {
    // No tmux server running
  }
}

/** Kill a tmux session by name, ignoring errors. */
function killTmuxSession(name: string): void {
  try {
    Bun.spawnSync(["tmux", "kill-session", "-t", name]);
  } catch {}
}

export async function attach(
  worktreeName: string,
  cols: number,
  rows: number
): Promise<string> {
  if (sessions.has(worktreeName)) {
    await detach(worktreeName);
  }

  const tmuxSession = await getTmuxSession();
  const gName = groupedName();
  const windowTarget = `wm-${worktreeName}`;

  // Kill stale session with same name if it exists (leftover from previous server run)
  killTmuxSession(gName);

  const cmd = [
    `tmux new-session -d -s "${gName}" -t "${tmuxSession}"`,
    `tmux select-window -t "${gName}:${windowTarget}"`,
    `stty rows ${rows} cols ${cols}`,
    `exec tmux attach-session -t "${gName}"`,
  ].join(" && ");

  const session: TerminalSession = {
    proc: null as any,
    groupedSessionName: gName,
    scrollback: [],
    onData: null,
    onExit: null,
  };

  sessions.set(worktreeName, session);

  const proc = Bun.spawn(["script", "-q", "-c", cmd, "/dev/null"], {
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, TERM: "xterm-256color" },
  });

  session.proc = proc;

  // Read stdout → push to scrollback + callback
  (async () => {
    const reader = proc.stdout.getReader();
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const str = new TextDecoder().decode(value);
        session.scrollback.push(str);
        if (session.scrollback.length > MAX_SCROLLBACK) {
          session.scrollback.shift();
        }
        session.onData?.(str);
      }
    } catch {
      // Stream closed
    }
  })();

  proc.exited.then((exitCode) => {
    session.onExit?.(exitCode);
    sessions.delete(worktreeName);
    killTmuxSession(gName);
  });

  return worktreeName;
}

export async function detach(worktreeName: string): Promise<void> {
  const session = sessions.get(worktreeName);
  if (!session) return;

  session.proc.kill();
  sessions.delete(worktreeName);

  killTmuxSession(session.groupedSessionName);
}

export function write(worktreeName: string, data: string): void {
  const session = sessions.get(worktreeName);
  if (session) {
    session.proc.stdin.write(new TextEncoder().encode(data));
  }
}

export function resize(worktreeName: string, cols: number, rows: number): void {
  const session = sessions.get(worktreeName);
  if (!session) return;
  // Resize via tmux directly (we don't have access to script's internal PTY)
  Bun.spawnSync(["tmux", "resize-window", "-t", session.groupedSessionName, "-x", String(cols), "-y", String(rows)]);
}

export function getScrollback(worktreeName: string): string {
  return sessions.get(worktreeName)?.scrollback.join("") ?? "";
}

export function setCallbacks(
  worktreeName: string,
  onData: (data: string) => void,
  onExit: (exitCode: number) => void
): void {
  const session = sessions.get(worktreeName);
  if (session) {
    session.onData = onData;
    session.onExit = onExit;
  }
}

export function clearCallbacks(worktreeName: string): void {
  const session = sessions.get(worktreeName);
  if (session) {
    session.onData = null;
    session.onExit = null;
  }
}

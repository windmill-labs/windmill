import { FileSink } from "bun";
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

function ts(): string {
  return new Date().toISOString().slice(11, 23);
}

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
  rows: number,
  initialPane?: number
): Promise<string> {
  console.log(`[term:${ts()}] attach(${worktreeName}) cols=${cols} rows=${rows} existing=${sessions.has(worktreeName)}`);
  if (sessions.has(worktreeName)) {
    console.log(`[term:${ts()}] attach(${worktreeName}) detaching existing session first`);
    await detach(worktreeName);
    console.log(`[term:${ts()}] attach(${worktreeName}) detach complete`);
  }

  const tmuxSession = await getTmuxSession();
  const gName = groupedName();
  const windowTarget = `wm-${worktreeName}`;
  console.log(`[term:${ts()}] attach(${worktreeName}) tmuxSession=${tmuxSession} gName=${gName} window=${windowTarget}`);

  // Kill stale session with same name if it exists (leftover from previous server run)
  killTmuxSession(gName);

  const paneTarget = `${gName}:${windowTarget}.${initialPane ?? 0}`;
  const cmd = [
    `tmux new-session -d -s "${gName}" -t "${tmuxSession}"`,
    `tmux set-option -t "${gName}" mouse on`,
    `tmux set-option -t "${gName}" set-clipboard on`,
    `tmux select-window -t "${gName}:${windowTarget}"`,
    // Unzoom if a previous session left a pane zoomed (zoom state is shared across grouped sessions)
    `if [ "$(tmux display-message -t '${gName}:${windowTarget}' -p '#{window_zoomed_flag}')" = "1" ]; then tmux resize-pane -Z -t '${gName}:${windowTarget}'; fi`,
    `tmux select-pane -t "${paneTarget}"`,
    // On mobile, zoom the selected pane to fill the window
    ...(initialPane !== undefined ? [`tmux resize-pane -Z -t "${paneTarget}"`] : []),
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
  console.log(`[term:${ts()}] attach(${worktreeName}) spawned pid=${proc.pid}`);

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
    console.log(`[term:${ts()}] proc exited(${worktreeName}) pid=${proc.pid} code=${exitCode}`);
    // Only clean up if this session is still the active one (not replaced by a new attach)
    if (sessions.get(worktreeName) === session) {
      session.onExit?.(exitCode);
      sessions.delete(worktreeName);
    } else {
      console.log(`[term:${ts()}] proc exited(${worktreeName}) stale session, skipping cleanup`);
    }
    killTmuxSession(gName);
  });

  return worktreeName;
}

export async function detach(worktreeName: string): Promise<void> {
  const session = sessions.get(worktreeName);
  if (!session) {
    console.log(`[term:${ts()}] detach(${worktreeName}) no session found`);
    return;
  }

  console.log(`[term:${ts()}] detach(${worktreeName}) killing pid=${session.proc.pid} tmux=${session.groupedSessionName}`);
  session.proc.kill();
  sessions.delete(worktreeName);

  killTmuxSession(session.groupedSessionName);
  console.log(`[term:${ts()}] detach(${worktreeName}) done`);
}

export function write(worktreeName: string, data: string): void {
  const session = sessions.get(worktreeName);
  if (!session) {
    console.log(`[term:${ts()}] write(${worktreeName}) NO SESSION - input dropped (${data.length} bytes)`);
    return;
  }
  if (!session.proc.stdin) {
    console.log(`[term:${ts()}] write(${worktreeName}) NO STDIN - input dropped (${data.length} bytes)`);
    return;
  }
  (session.proc.stdin as FileSink).write(new TextEncoder().encode(data));
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

export function selectPane(worktreeName: string, paneIndex: number): void {
  const session = sessions.get(worktreeName);
  if (!session) {
    console.log(`[term:${ts()}] selectPane(${worktreeName}) no session found`);
    return;
  }
  const windowTarget = `wm-${worktreeName}`;
  const target = `${session.groupedSessionName}:${windowTarget}.${paneIndex}`;
  console.log(`[term:${ts()}] selectPane(${worktreeName}) pane=${paneIndex} target=${target}`);
  const r1 = Bun.spawnSync(["tmux", "select-pane", "-t", target]);
  const r2 = Bun.spawnSync(["tmux", "resize-pane", "-Z", "-t", target]);
  console.log(`[term:${ts()}] selectPane(${worktreeName}) select=${r1.exitCode} zoom=${r2.exitCode}`);
}

export function clearCallbacks(worktreeName: string): void {
  const session = sessions.get(worktreeName);
  if (session) {
    session.onData = null;
    session.onExit = null;
  }
}

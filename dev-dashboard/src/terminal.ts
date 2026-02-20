import { getTmuxSession } from "./workmux";

interface TerminalSession {
  proc: ReturnType<typeof Bun.spawn>;
  groupedSessionName: string;
  scrollback: string[];
  onData: ((data: string) => void) | null;
  onExit: ((exitCode: number) => void) | null;
}

const MAX_SCROLLBACK = 5000;
const sessions = new Map<string, TerminalSession>();
let sessionCounter = 0;

function groupedName(): string {
  return `wm-dash-${++sessionCounter}`;
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

  // Create a grouped tmux session (independent sizing) and attach to the worktree's window
  const cmd = [
    `tmux new-session -d -s "${gName}" -t "${tmuxSession}"`,
    `tmux select-window -t "${gName}:${windowTarget}"`,
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

  const proc = Bun.spawn(["bash", "-c", cmd], {
    terminal: {
      cols,
      rows,
      name: "xterm-256color",
      data(_terminal, data) {
        const str = typeof data === "string" ? data : new TextDecoder().decode(data);
        session.scrollback.push(str);
        if (session.scrollback.length > MAX_SCROLLBACK) {
          session.scrollback.shift();
        }
        session.onData?.(str);
      },
      exit() {},
    },
  });

  session.proc = proc;

  proc.exited.then((exitCode) => {
    session.onExit?.(exitCode);
    sessions.delete(worktreeName);
    try {
      Bun.spawnSync(["tmux", "kill-session", "-t", gName]);
    } catch {
      // Session may already be gone
    }
  });

  return worktreeName;
}

export async function detach(worktreeName: string): Promise<void> {
  const session = sessions.get(worktreeName);
  if (!session) return;

  session.proc.kill();
  sessions.delete(worktreeName);

  try {
    Bun.spawnSync(["tmux", "kill-session", "-t", session.groupedSessionName]);
  } catch {
    // Already gone
  }
}

export function write(worktreeName: string, data: string): void {
  sessions.get(worktreeName)?.proc.terminal?.write(data);
}

export function resize(worktreeName: string, cols: number, rows: number): void {
  sessions.get(worktreeName)?.proc.terminal?.resize(cols, rows);
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

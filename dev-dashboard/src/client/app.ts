import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";

// --- Types ---

interface WorktreeInfo {
  branch: string;
  agent: string;
  mux: string;
  path: string;
  status: string;
  elapsed: string;
  title: string;
}

// --- State ---

let worktrees: WorktreeInfo[] = [];
let selectedWorktree: string | null = null;
let ws: WebSocket | null = null;
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;

// --- DOM refs ---

const worktreeList = document.getElementById("worktree-list") as HTMLUListElement;
const wtName = document.getElementById("wt-name") as HTMLSpanElement;
const wtActions = document.getElementById("wt-actions") as HTMLDivElement;
const wtStatusBadge = document.getElementById("wt-status-badge") as HTMLSpanElement;
const terminalContainer = document.getElementById("terminal-container") as HTMLDivElement;
const placeholder = document.getElementById("placeholder") as HTMLDivElement;

const btnNew = document.getElementById("btn-new") as HTMLButtonElement;
const btnOpen = document.getElementById("btn-open") as HTMLButtonElement;
const btnClose = document.getElementById("btn-close") as HTMLButtonElement;
const btnSend = document.getElementById("btn-send") as HTMLButtonElement;
const btnRemove = document.getElementById("btn-remove") as HTMLButtonElement;

const newDialog = document.getElementById("new-dialog") as HTMLDialogElement;
const sendDialog = document.getElementById("send-dialog") as HTMLDialogElement;

// --- API helpers ---

async function api<T = unknown>(path: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(`/api/${path}`, {
    headers: { "Content-Type": "application/json" },
    ...opts,
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data as T;
}

// --- Worktree list ---

async function refreshWorktrees(): Promise<void> {
  try {
    worktrees = await api<WorktreeInfo[]>("worktrees");
    renderWorktreeList();
  } catch (err) {
    console.error("Failed to refresh worktrees:", err);
  }
}

function statusDotClass(agent: string): string {
  if (agent === "working") return "working";
  if (agent === "waiting") return "waiting";
  if (agent === "error") return "error";
  return "stopped";
}

function renderWorktreeList(): void {
  worktreeList.innerHTML = "";
  for (const wt of worktrees) {
    const li = document.createElement("li");
    if (wt.branch === selectedWorktree) li.classList.add("active");

    const isMain = wt.path === "(here)" || wt.branch === "main";

    li.innerHTML = `
      <span class="wt-branch">${escapeHtml(wt.branch)}</span>
      <span class="wt-meta">
        <span><span class="status-dot ${statusDotClass(wt.agent)}"></span>${escapeHtml(wt.agent || "none")}</span>
        ${wt.mux && wt.mux !== "-" ? `<span>mux: ${escapeHtml(wt.mux)}</span>` : ""}
        ${isMain ? "<span>main</span>" : ""}
      </span>
    `;

    li.addEventListener("click", () => selectWorktree(wt.branch));
    worktreeList.appendChild(li);
  }
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

// --- Terminal ---

function selectWorktree(branch: string): void {
  if (selectedWorktree === branch) return;
  selectedWorktree = branch;

  // Update sidebar selection
  renderWorktreeList();

  // Update top bar
  const wt = worktrees.find(w => w.branch === branch);
  wtName.textContent = branch;
  wtActions.classList.remove("hidden");
  wtStatusBadge.textContent = wt?.status || wt?.agent || "";

  // Only connect terminal for non-main worktrees that have a tmux window
  const isMain = wt?.path === "(here)" || branch === "main";
  if (isMain) {
    disconnectTerminal();
    placeholder.classList.remove("hidden");
    placeholder.querySelector("p")!.textContent = "Main worktree — use workmux to manage";
    terminalContainer.classList.remove("visible");
    return;
  }

  connectTerminal(branch);
}

function connectTerminal(worktree: string): void {
  disconnectTerminal();

  // Show terminal container
  placeholder.classList.add("hidden");
  terminalContainer.classList.add("visible");

  // Create terminal
  term = new Terminal({
    cursorBlink: true,
    theme: {
      background: "#0d1117",
      foreground: "#e6edf3",
      cursor: "#58a6ff",
      selectionBackground: "#264f78",
    },
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Menlo, monospace",
    fontSize: 13,
    scrollback: 10000,
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.loadAddon(new WebLinksAddon());
  term.open(terminalContainer);

  // Delay fit() so the container has its final dimensions after display:none → block
  requestAnimationFrame(() => {
    fitAddon?.fit();
  });

  // WebSocket connection
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  ws = new WebSocket(`${protocol}//${location.host}/ws/${encodeURIComponent(worktree)}`);

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data);
      switch (msg.type) {
        case "scrollback":
        case "output":
          term?.write(msg.data);
          break;
        case "exit":
          term?.writeln(`\r\n\x1b[33m[Process exited with code ${msg.exitCode}]\x1b[0m`);
          break;
        case "error":
          term?.writeln(`\r\n\x1b[31m[Error: ${msg.message}]\x1b[0m`);
          break;
      }
    } catch {
      // Ignore malformed messages
    }
  };

  ws.onopen = () => {
    // Send actual fitted dimensions once connected
    if (term && fitAddon) {
      fitAddon.fit();
      ws!.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
    }
  };

  ws.onclose = () => {
    term?.writeln("\r\n\x1b[90m[Disconnected]\x1b[0m");
  };

  // Terminal input → WebSocket
  term.onData((data) => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data }));
    }
  });

  // Handle resize
  resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit();
    if (term && ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
    }
  });
  resizeObserver.observe(terminalContainer);
}

function disconnectTerminal(): void {
  resizeObserver?.disconnect();
  resizeObserver = null;
  ws?.close();
  ws = null;
  term?.dispose();
  term = null;
  fitAddon = null;
  terminalContainer.innerHTML = "";
}

// --- Actions ---

btnNew.addEventListener("click", () => {
  (document.getElementById("new-branch") as HTMLInputElement).value = "";
  (document.getElementById("new-prompt") as HTMLTextAreaElement).value = "";
  newDialog.showModal();
});

newDialog.querySelector("form")!.addEventListener("submit", async (e) => {
  e.preventDefault();
  const branch = (document.getElementById("new-branch") as HTMLInputElement).value.trim();
  const prompt = (document.getElementById("new-prompt") as HTMLTextAreaElement).value.trim();

  if (!branch) return;

  try {
    newDialog.close();
    await api("worktrees", {
      method: "POST",
      body: JSON.stringify({ branch, prompt: prompt || undefined }),
    });
    await refreshWorktrees();
    selectWorktree(branch);
  } catch (err) {
    alert(`Failed to create worktree: ${err instanceof Error ? err.message : err}`);
  }
});

btnOpen.addEventListener("click", async () => {
  if (!selectedWorktree) return;
  try {
    await api(`worktrees/${encodeURIComponent(selectedWorktree)}/open`, { method: "POST" });
    await refreshWorktrees();
    // Reconnect terminal since tmux window is now open
    connectTerminal(selectedWorktree);
  } catch (err) {
    alert(`Failed to open: ${err instanceof Error ? err.message : err}`);
  }
});

btnClose.addEventListener("click", async () => {
  if (!selectedWorktree) return;
  try {
    await api(`worktrees/${encodeURIComponent(selectedWorktree)}/close`, { method: "POST" });
    disconnectTerminal();
    placeholder.classList.remove("hidden");
    placeholder.querySelector("p")!.textContent = "Worktree closed";
    terminalContainer.classList.remove("visible");
    await refreshWorktrees();
  } catch (err) {
    alert(`Failed to close: ${err instanceof Error ? err.message : err}`);
  }
});

btnSend.addEventListener("click", () => {
  (document.getElementById("send-prompt") as HTMLTextAreaElement).value = "";
  sendDialog.showModal();
});

sendDialog.querySelector("form")!.addEventListener("submit", async (e) => {
  e.preventDefault();
  const prompt = (document.getElementById("send-prompt") as HTMLTextAreaElement).value.trim();
  if (!prompt || !selectedWorktree) return;

  try {
    sendDialog.close();
    await api(`worktrees/${encodeURIComponent(selectedWorktree)}/send`, {
      method: "POST",
      body: JSON.stringify({ prompt }),
    });
  } catch (err) {
    alert(`Failed to send prompt: ${err instanceof Error ? err.message : err}`);
  }
});

btnRemove.addEventListener("click", async () => {
  if (!selectedWorktree) return;
  if (!confirm(`Remove worktree "${selectedWorktree}"? This also deletes the branch.`)) return;

  try {
    disconnectTerminal();
    await api(`worktrees/${encodeURIComponent(selectedWorktree)}`, { method: "DELETE" });
    selectedWorktree = null;
    wtName.textContent = "Select a worktree";
    wtActions.classList.add("hidden");
    placeholder.classList.remove("hidden");
    placeholder.querySelector("p")!.textContent = "Select a worktree from the sidebar to connect";
    terminalContainer.classList.remove("visible");
    await refreshWorktrees();
  } catch (err) {
    alert(`Failed to remove: ${err instanceof Error ? err.message : err}`);
  }
});

// --- Init ---

refreshWorktrees();
setInterval(refreshWorktrees, 5000);

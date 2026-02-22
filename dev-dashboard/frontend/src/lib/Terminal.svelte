<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";

  let { worktree }: { worktree: string } = $props();

  let containerEl: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let ws: WebSocket;
  let resizeObs: ResizeObserver;

  onMount(() => {
    term = new Terminal({
      cursorBlink: true,
      theme: {
        background: "#0d1117",
        foreground: "#e6edf3",
        cursor: "#58a6ff",
        selectionBackground: "#264f78",
      },
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Menlo, monospace",
      fontSize: 11,
      scrollback: 10000,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.open(containerEl);

    // Prevent browser context menu so tmux right-click works unobstructed
    containerEl.addEventListener("contextmenu", (e) => e.preventDefault());

    // Handle OSC 52 sequences from tmux → write to system clipboard
    term.parser.registerOscHandler(52, (data) => {
      const idx = data.indexOf(";");
      if (idx !== -1) {
        const b64 = data.slice(idx + 1);
        try {
          const text = atob(b64);
          navigator.clipboard.writeText(text);
        } catch {}
      }
      return true;
    });

    // Auto-copy on xterm.js selection (e.g. when user Shift+drags to bypass tmux mouse)
    term.onSelectionChange(() => {
      const sel = term.getSelection();
      if (sel) {
        navigator.clipboard.writeText(sel);
      }
    });

    // Let app-level shortcuts (Cmd+Arrow, Cmd+N, Cmd+D) bubble up instead of
    // being consumed by xterm.  Return false → xterm ignores the event.
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.type !== "keydown") return true;
      const mod = e.metaKey || e.ctrlKey;
      if (mod && (e.key === "ArrowUp" || e.key === "ArrowDown")) return false;
      if (mod && (e.key === "k" || e.key === "K")) return false;
      if (mod && (e.key === "d" || e.key === "D")) return false;
      return true;
    });

    requestAnimationFrame(() => {
      fitAddon.fit();
      term.focus();
    });

    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(`${protocol}//${location.host}/ws/${encodeURIComponent(worktree)}`);

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        switch (msg.type) {
          case "scrollback":
          case "output":
            term.write(msg.data);
            break;
          case "exit":
            term.writeln(`\r\n\x1b[33m[Process exited with code ${msg.exitCode}]\x1b[0m`);
            break;
          case "error":
            term.writeln(`\r\n\x1b[31m[Error: ${msg.message}]\x1b[0m`);
            break;
        }
      } catch {
        // Ignore malformed messages
      }
    };

    ws.onopen = () => {
      fitAddon.fit();
      ws.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
    };

    ws.onclose = () => {
      term.writeln("\r\n\x1b[90m[Disconnected]\x1b[0m");
    };

    term.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    });

    resizeObs = new ResizeObserver(() => {
      fitAddon.fit();
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
      }
    });
    resizeObs.observe(containerEl);
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    ws?.close();
    term?.dispose();
  });
</script>

<div class="flex-1 min-h-0 w-full p-1 overflow-hidden" bind:this={containerEl}></div>

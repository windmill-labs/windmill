<script lang="ts">
  import type { WorktreeInfo } from "./types";

  let { name, worktree, sshHost, onremove, onsettings }: {
    name: string | null;
    worktree: WorktreeInfo | undefined;
    sshHost: string;
    onremove: () => void;
    onsettings: () => void;
  } = $props();

  let cursorUrl = $derived.by(() => {
    const dir = worktree?.dir;
    if (!dir) return null;
    if (sshHost) {
      return `cursor://vscode-remote/ssh-remote+${sshHost}${dir}`;
    }
    return `cursor://file${dir}`;
  });

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<div class="flex items-center justify-between px-4 py-2 bg-topbar border-b border-edge min-h-12">
  <div class="flex items-center gap-3">
    <span class="text-sm font-semibold">{name ?? "Select a worktree"}</span>
    {#if worktree?.backendPort}
      <a
        href="{window.location.protocol}//{window.location.hostname}:{worktree.backendPort}"
        target="_blank"
        rel="noopener"
        class="text-[11px] px-1.5 py-0.5 rounded border font-mono no-underline hover:opacity-80 {worktree.backendRunning ? 'text-success border-success/40' : 'text-muted border-edge pointer-events-none'}"
      >BE :{worktree.backendPort}</a>
    {/if}
    {#if worktree?.frontendPort}
      <a
        href="{window.location.protocol}//{window.location.hostname}:{worktree.frontendPort}"
        target="_blank"
        rel="noopener"
        class="text-[11px] px-1.5 py-0.5 rounded border font-mono no-underline hover:opacity-80 {worktree.frontendRunning ? 'text-success border-success/40' : 'text-muted border-edge pointer-events-none'}"
      >FE :{worktree.frontendPort}</a>
    {/if}
    {#if cursorUrl}
      <a
        href={cursorUrl}
        class="text-[11px] px-1.5 py-0.5 rounded-l border border-accent/40 text-accent font-mono no-underline hover:opacity-80"
        title="Open in Cursor"
      >Cursor</a><button
        type="button"
        class="text-[11px] px-1 py-0.5 rounded-r border border-l-0 border-accent/40 text-accent cursor-pointer bg-transparent hover:opacity-80"
        title="Cursor SSH settings"
        onclick={onsettings}
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
      </button>
    {/if}
  </div>
  {#if name}
    <div class="flex gap-2 items-center">
      <span class="text-xs px-2 py-0.5 rounded-xl bg-hover">{worktree?.status || worktree?.agent || ""}</span>
      <button class="{btn} !text-danger !border-danger hover:!bg-danger/10" onclick={onremove} title="Remove worktree">Remove</button>
    </div>
  {/if}
</div>

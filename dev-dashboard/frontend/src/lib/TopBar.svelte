<script lang="ts">
  import type { WorktreeInfo } from "./types";

  let { name, worktree, sshHost, onremove }: {
    name: string | null;
    worktree: WorktreeInfo | undefined;
    sshHost: string;
    onremove: () => void;
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
        class="text-[11px] px-1.5 py-0.5 rounded border border-accent/40 text-accent font-mono no-underline hover:opacity-80"
        title="Open in Cursor"
      >Cursor</a>
    {/if}
  </div>
  {#if name}
    <div class="flex gap-2 items-center">
      <span class="text-xs px-2 py-0.5 rounded-xl bg-hover">{worktree?.status || worktree?.agent || ""}</span>
      <button class="{btn} !text-danger !border-danger hover:!bg-danger/10" onclick={onremove} title="Remove worktree">Remove</button>
    </div>
  {/if}
</div>

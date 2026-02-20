<script lang="ts">
  import type { WorktreeInfo } from "./types";

  let { name, worktree, onopen, onclose, onsend, onremove }: {
    name: string | null;
    worktree: WorktreeInfo | undefined;
    onopen: () => void;
    onclose: () => void;
    onsend: () => void;
    onremove: () => void;
  } = $props();

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<div class="flex items-center justify-between px-4 py-2 bg-topbar border-b border-edge min-h-12">
  <span class="text-sm font-semibold">{name ?? "Select a worktree"}</span>
  {#if name}
    <div class="flex gap-2 items-center">
      <span class="text-xs px-2 py-0.5 rounded-xl bg-hover">{worktree?.status || worktree?.agent || ""}</span>
      <button class={btn} onclick={onopen} title="Open tmux window">Open</button>
      <button class={btn} onclick={onclose} title="Close tmux window">Close</button>
      <button class={btn} onclick={onsend} title="Send prompt to agent">Send</button>
      <button class="{btn} !text-danger !border-danger hover:!bg-danger/10" onclick={onremove} title="Remove worktree">Remove</button>
    </div>
  {/if}
</div>

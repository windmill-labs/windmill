<script lang="ts">
  import { onMount } from "svelte";
  import WorktreeList from "./lib/WorktreeList.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import SendDialog from "./lib/SendDialog.svelte";
  import type { WorktreeInfo } from "./lib/types";
  import * as api from "./lib/api";

  let worktrees = $state<WorktreeInfo[]>([]);
  let selectedBranch = $state<string | null>(null);
  let showSendDialog = $state(false);
  let creating = $state(false);

  let selectedWorktree = $derived(worktrees.find((w) => w.branch === selectedBranch));
  let isMain = $derived(selectedWorktree?.path === "(here)" || selectedBranch === "main");

  async function refresh() {
    try {
      worktrees = await api.fetchWorktrees();
    } catch (err) {
      console.error("Failed to refresh:", err);
    }
  }

  function randomName(len: number): string {
    const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
    let result = "";
    for (let i = 0; i < len; i++) {
      result += chars[Math.floor(Math.random() * chars.length)];
    }
    return result;
  }

  async function handleNew() {
    const branch = randomName(8);
    creating = true;
    try {
      await api.createWorktree(branch);
      await api.openWorktree(branch);
      await refresh();
      selectedBranch = branch;
    } catch (err) {
      alert(`Failed to create: ${err instanceof Error ? err.message : err}`);
    } finally {
      creating = false;
    }
  }

  async function handleOpen() {
    if (!selectedBranch) return;
    try {
      await api.openWorktree(selectedBranch);
      await refresh();
    } catch (err) {
      alert(`Failed to open: ${err instanceof Error ? err.message : err}`);
    }
  }

  async function handleClose() {
    if (!selectedBranch) return;
    try {
      await api.closeWorktree(selectedBranch);
      await refresh();
    } catch (err) {
      alert(`Failed to close: ${err instanceof Error ? err.message : err}`);
    }
  }

  async function handleRemove() {
    if (!selectedBranch || !confirm(`Remove worktree "${selectedBranch}"?`)) return;
    try {
      await api.removeWorktree(selectedBranch);
      selectedBranch = null;
      await refresh();
    } catch (err) {
      alert(`Failed to remove: ${err instanceof Error ? err.message : err}`);
    }
  }

  async function handleSend(prompt: string) {
    if (!selectedBranch) return;
    showSendDialog = false;
    try {
      await api.sendPrompt(selectedBranch, prompt);
    } catch (err) {
      alert(`Failed to send: ${err instanceof Error ? err.message : err}`);
    }
  }

  onMount(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex h-screen bg-surface text-primary">
  <aside class="w-[260px] min-w-[260px] bg-sidebar border-r border-edge flex flex-col overflow-hidden">
    <div class="flex items-center justify-between p-4 border-b border-edge">
      <h1 class="text-base font-semibold">Windmill</h1>
      <button
        class="w-8 h-8 rounded-md border border-edge bg-surface text-accent text-lg flex items-center justify-center p-0 cursor-pointer hover:bg-hover disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={handleNew}
        disabled={creating}
        title="New Worktree"
      >+</button>
    </div>
    <WorktreeList {worktrees} selected={selectedBranch} onselect={(b) => (selectedBranch = b)} />
  </aside>

  <main class="flex-1 flex flex-col overflow-hidden">
    <TopBar
      name={selectedBranch}
      worktree={selectedWorktree}
      onopen={handleOpen}
      onclose={handleClose}
      onsend={() => (showSendDialog = true)}
      onremove={handleRemove}
    />

    {#if selectedBranch && !isMain}
      {#key selectedBranch}
        <Terminal worktree={selectedBranch} />
      {/key}
    {:else}
      <div class="flex-1 flex items-center justify-center text-muted text-sm">
        <p>
          {#if isMain}
            Main worktree — use workmux to manage
          {:else}
            Select a worktree from the sidebar to connect
          {/if}
        </p>
      </div>
    {/if}
  </main>
</div>

{#if showSendDialog}
  <SendDialog onsubmit={handleSend} oncancel={() => (showSendDialog = false)} />
{/if}

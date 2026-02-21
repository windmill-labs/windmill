<script lang="ts">
  import { onMount } from "svelte";
  import WorktreeList from "./lib/WorktreeList.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import ConfirmDialog from "./lib/ConfirmDialog.svelte";
  import type { WorktreeInfo } from "./lib/types";
  import * as api from "./lib/api";

  let worktrees = $state<WorktreeInfo[]>([]);
  let selectedBranch = $state<string | null>(null);
  let showConfirmRemove = $state(false);
  let creating = $state(false);
  let removing = $state(false);

  let selectedWorktree = $derived(worktrees.find((w) => w.branch === selectedBranch));
  let hasMux = $derived(selectedWorktree?.mux === "✓");
  let isMain = $derived(selectedWorktree?.path === "(here)" || selectedBranch === "main");
  let canConnect = $derived(!!selectedBranch && !isMain && hasMux);

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

  async function handleRemove() {
    if (!selectedBranch) return;
    removing = true;
    try {
      await api.removeWorktree(selectedBranch);
      showConfirmRemove = false;
      selectedBranch = null;
      await refresh();
    } catch (err) {
      alert(`Failed to remove: ${err instanceof Error ? err.message : err}`);
    } finally {
      removing = false;
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
        class="h-8 px-2 gap-1.5 rounded-md border border-edge bg-surface text-accent text-xs flex items-center justify-center cursor-pointer hover:bg-hover disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={handleNew}
        disabled={creating}
        title="New Worktree"
      >{#if creating}<span class="spinner"></span>{:else}<span class="text-lg leading-none">+</span>{/if} Create</button>
    </div>
    <WorktreeList {worktrees} selected={selectedBranch} onselect={(b) => (selectedBranch = b)} />
  </aside>

  <main class="flex-1 min-w-0 flex flex-col overflow-hidden">
    <TopBar
      name={selectedBranch}
      worktree={selectedWorktree}
      onremove={() => (showConfirmRemove = true)}
    />

    {#if canConnect}
      {#key selectedBranch}
        <Terminal worktree={selectedBranch} />
      {/key}
    {:else}
      <div class="flex-1 flex items-center justify-center text-muted text-sm">
        <p>
          {#if isMain}
            Main worktree — use workmux to manage
          {:else if selectedBranch && !hasMux}
            No tmux window for this worktree
          {:else}
            Select a worktree from the sidebar to connect
          {/if}
        </p>
      </div>
    {/if}
  </main>
</div>

{#if showConfirmRemove}
  <ConfirmDialog
    message={`Remove worktree "${selectedBranch}"? This action cannot be undone.`}
    loading={removing}
    onconfirm={handleRemove}
    oncancel={() => (showConfirmRemove = false)}
  />
{/if}

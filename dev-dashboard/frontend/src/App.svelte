<script lang="ts">
  import { onMount } from "svelte";
  import WorktreeList from "./lib/WorktreeList.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import ConfirmDialog from "./lib/ConfirmDialog.svelte";
  import CreateWorktreeDialog from "./lib/CreateWorktreeDialog.svelte";
  import type { WorktreeInfo } from "./lib/types";
  import type { Profile, Agent } from "./lib/api";
  import * as api from "./lib/api";

  let worktrees = $state<WorktreeInfo[]>([]);
  let selectedBranch = $state<string | null>(null);
  let removeBranch = $state<string | null>(null);
  let removingBranches = $state<Set<string>>(new Set());
  let showCreateDialog = $state(false);
  let creating = $state(false);

  let visibleWorktrees = $derived(
    worktrees.filter((w) => w.path === "(here)" || w.branch === "main" || w.mux === "✓")
  );
  let selectedWorktree = $derived(visibleWorktrees.find((w) => w.branch === selectedBranch));
  let isMain = $derived(selectedWorktree?.path === "(here)" || selectedBranch === "main");
  let canConnect = $derived(!!selectedBranch && !isMain);

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

  async function handleCreate(name: string, profile: Profile, agent: Agent) {
    const branch = name || randomName(8);
    creating = true;
    try {
      await api.createWorktree(branch, profile, agent);
      await api.openWorktree(branch);
      showCreateDialog = false;
      await refresh();
      selectedBranch = branch;
    } catch (err) {
      alert(`Failed to create: ${err instanceof Error ? err.message : err}`);
    } finally {
      creating = false;
    }
  }

  async function handleRemove() {
    const branch = removeBranch;
    if (!branch) return;
    removeBranch = null;

    // Select neighbor before starting the async removal
    if (selectedBranch === branch) {
      const idx = visibleWorktrees.findIndex((w) => w.branch === branch);
      const neighbor = visibleWorktrees[idx - 1] ?? visibleWorktrees[idx + 1];
      const isNeighborMain = neighbor && (neighbor.path === "(here)" || neighbor.branch === "main");
      selectedBranch = neighbor && !isNeighborMain ? neighbor.branch : null;
    }

    removingBranches = new Set([...removingBranches, branch]);
    try {
      await api.removeWorktree(branch);
      await refresh();
    } catch (err) {
      alert(`Failed to remove: ${err instanceof Error ? err.message : err}`);
    } finally {
      removingBranches = new Set([...removingBranches].filter((b) => b !== branch));
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
        onclick={() => (showCreateDialog = true)}
        disabled={creating}
        title="New Worktree"
      ><span class="text-lg leading-none">+</span> New</button>
    </div>
    <WorktreeList worktrees={visibleWorktrees} selected={selectedBranch} removing={removingBranches} onselect={(b) => (selectedBranch = b)} onremove={(b) => (removeBranch = b)} />
  </aside>

  <main class="flex-1 min-w-0 flex flex-col overflow-hidden">
    <TopBar
      name={selectedBranch}
      worktree={selectedWorktree}
      onremove={() => { if (selectedBranch) removeBranch = selectedBranch; }}
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
          {:else}
            Select a worktree from the sidebar to connect
          {/if}
        </p>
      </div>
    {/if}
  </main>
</div>

{#if showCreateDialog}
  <CreateWorktreeDialog
    loading={creating}
    oncreate={handleCreate}
    oncancel={() => (showCreateDialog = false)}
  />
{/if}

{#if removeBranch}
  <ConfirmDialog
    message={`Remove worktree "${removeBranch}"? This action cannot be undone.`}
    onconfirm={handleRemove}
    oncancel={() => (removeBranch = null)}
  />
{/if}

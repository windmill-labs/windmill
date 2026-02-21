<script lang="ts">
  import { onMount } from "svelte";
  import WorktreeList from "./lib/WorktreeList.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import ConfirmDialog from "./lib/ConfirmDialog.svelte";
  import type { WorktreeInfo } from "./lib/types";
  import type { Profile } from "./lib/api";
  import * as api from "./lib/api";

  const PROFILES: { value: Profile; label: string }[] = [
    { value: "agent-only", label: "Agent only" },
    { value: "agent-yolo", label: "Agent (skip permissions)" },
    { value: "full", label: "Full (agent + backend + frontend)" },
  ];

  let worktrees = $state<WorktreeInfo[]>([]);
  let selectedBranch = $state<string | null>(null);
  let removeBranch = $state<string | null>(null);
  let showCreateDialog = $state(false);
  let creating = $state(false);
  let removing = $state(false);
  let createProfile = $state<Profile>("agent-only");

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

  async function handleCreate() {
    const branch = randomName(8);
    creating = true;
    try {
      await api.createWorktree(branch, createProfile);
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
    if (!removeBranch) return;
    removing = true;
    try {
      await api.removeWorktree(removeBranch);
      if (selectedBranch === removeBranch) selectedBranch = null;
      removeBranch = null;
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
        onclick={() => (showCreateDialog = true)}
        disabled={creating}
        title="New Worktree"
      ><span class="text-lg leading-none">+</span> New</button>
    </div>
    <WorktreeList worktrees={visibleWorktrees} selected={selectedBranch} onselect={(b) => (selectedBranch = b)} onremove={(b) => (removeBranch = b)} />
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
  {@const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover"}
  <dialog
    open
    class="fixed inset-0 z-50 bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[380px] w-[90%] m-auto"
  >
    <h2 class="text-base mb-4">New Worktree</h2>
    <div class="flex flex-col gap-2 mb-6">
      {#each PROFILES as profile}
        <label
          class="flex items-center gap-2.5 p-2.5 rounded-lg border cursor-pointer text-[13px] transition-colors
            {createProfile === profile.value ? 'border-accent bg-accent/10' : 'border-edge hover:bg-hover'}"
        >
          <input
            type="radio"
            name="profile"
            value={profile.value}
            checked={createProfile === profile.value}
            onchange={() => (createProfile = profile.value)}
            class="accent-[var(--accent)]"
          />
          {profile.label}
        </label>
      {/each}
    </div>
    <div class="flex justify-end gap-2">
      <button type="button" class={btn} onclick={() => (showCreateDialog = false)} disabled={creating}>Cancel</button>
      <button
        type="button"
        class="{btn} !bg-accent !text-white !border-accent hover:!opacity-90 disabled:!opacity-50 disabled:!cursor-not-allowed flex items-center gap-1.5"
        onclick={handleCreate}
        disabled={creating}
      >{#if creating}<span class="spinner"></span>{/if} Create</button>
    </div>
  </dialog>
  <div class="fixed inset-0 bg-black/50 z-40" onclick={() => (showCreateDialog = false)}></div>
{/if}

{#if removeBranch}
  <ConfirmDialog
    message={`Remove worktree "${removeBranch}"? This action cannot be undone.`}
    loading={removing}
    onconfirm={handleRemove}
    oncancel={() => (removeBranch = null)}
  />
{/if}

<script lang="ts">
  import { onMount } from "svelte";
  import WorktreeList from "./lib/WorktreeList.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import ConfirmDialog from "./lib/ConfirmDialog.svelte";
  import CreateWorktreeDialog from "./lib/CreateWorktreeDialog.svelte";
  import SettingsDialog from "./lib/SettingsDialog.svelte";
  import PaneBar from "./lib/PaneBar.svelte";
  import type { WorktreeInfo } from "./lib/types";
  import type { Profile, Agent } from "./lib/api";
  import * as api from "./lib/api";

  let worktrees = $state<WorktreeInfo[]>([]);
  let selectedBranch = $state<string | null>(null);
  let removeBranch = $state<string | null>(null);
  let mergeBranch = $state<string | null>(null);
  let merging = $state(false);
  let mergeError = $state("");
  let removingBranches = $state<Set<string>>(new Set());
  const SSH_STORAGE_KEY = "wt-ssh-host";
  let showCreateDialog = $state(false);
  let showSettingsDialog = $state(false);
  let creating = $state(false);
  let sshHost = $state(localStorage.getItem(SSH_STORAGE_KEY) ?? "");

  // Mobile state
  let isMobile = $state(false);
  let sidebarOpen = $state(false);
  let activePane = $state(0);
  let terminalRef: { sendSelectPane: (pane: number) => void } | undefined = $state();

  let visibleWorktrees = $derived(
    worktrees.filter((w) => w.path === "(here)" || w.branch === "main" || w.mux === "✓")
  );
  let selectedWorktree = $derived(visibleWorktrees.find((w) => w.branch === selectedBranch));
  let isMain = $derived(selectedWorktree?.path === "(here)" || selectedBranch === "main");
  let canConnect = $derived(!!selectedBranch && !isMain);

  let paneBarProfile = $derived(
    selectedWorktree?.profile === "full" || selectedWorktree?.profile === "agent-yolo"
      ? selectedWorktree.profile as "full" | "agent-yolo"
      : null
  );
  let showPaneBar = $derived(isMobile && canConnect && paneBarProfile !== null);

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

  /** Sanitize user input into a valid git branch name */
  function sanitizeBranchName(raw: string): string {
    return raw
      .replace(/\s+/g, "-")              // spaces → dashes
      .replace(/[~^:?*\[\]\\]+/g, "")    // remove git-invalid chars
      .replace(/\.{2,}/g, ".")            // collapse ".." → "."
      .replace(/\/{2,}/g, "/")            // collapse consecutive slashes
      .replace(/-{2,}/g, "-")             // collapse consecutive dashes
      .replace(/^[.\-/]+|[.\-/]+$/g, "") // no leading/trailing . - /
      .replace(/\.lock$/i, "");           // no trailing .lock
  }

  async function handleCreate(name: string, profile: Profile, agent: Agent) {
    const branch = (name && sanitizeBranchName(name)) || randomName(8);
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

  function selectNeighborOf(branch: string) {
    if (selectedBranch !== branch) return;
    const idx = visibleWorktrees.findIndex((w) => w.branch === branch);
    const neighbor = visibleWorktrees[idx - 1] ?? visibleWorktrees[idx + 1];
    const isNeighborMain = neighbor && (neighbor.path === "(here)" || neighbor.branch === "main");
    selectedBranch = neighbor && !isNeighborMain ? neighbor.branch : null;
  }

  async function handleRemove() {
    const branch = removeBranch;
    if (!branch) return;
    removeBranch = null;
    selectNeighborOf(branch);

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

  async function handleMerge() {
    const branch = mergeBranch;
    if (!branch) return;

    merging = true;
    mergeError = "";
    try {
      await api.mergeWorktree(branch);
      mergeBranch = null;
      selectNeighborOf(branch);
      await refresh();
    } catch (err) {
      mergeError = err instanceof Error ? err.message : String(err);
    } finally {
      merging = false;
    }
  }

  function selectNeighborWorktree(direction: -1 | 1) {
    const selectable = visibleWorktrees.filter(
      (w) => w.path !== "(here)" && w.branch !== "main" && !removingBranches.has(w.branch)
    );
    if (selectable.length === 0) return;
    if (!selectedBranch) {
      selectedBranch = selectable[direction === 1 ? 0 : selectable.length - 1].branch;
      return;
    }
    const idx = selectable.findIndex((w) => w.branch === selectedBranch);
    const next = idx + direction;
    if (next >= 0 && next < selectable.length) {
      selectedBranch = selectable[next].branch;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // Ignore shortcuts when a dialog is open (let dialog handle its own keys)
    if (showCreateDialog || removeBranch || mergeBranch) return;

    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;

    if (e.key === "ArrowUp") {
      e.preventDefault();
      selectNeighborWorktree(-1);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      selectNeighborWorktree(1);
    } else if (e.key === "k" || e.key === "K") {
      e.preventDefault();
      if (!creating) showCreateDialog = true;
    } else if (e.key === "m" || e.key === "M") {
      e.preventDefault();
      if (selectedBranch && !isMain) mergeBranch = selectedBranch;
    } else if (e.key === "d" || e.key === "D") {
      e.preventDefault();
      if (selectedBranch && !isMain) removeBranch = selectedBranch;
    }
  }

  function handlePaneSelect(pane: number) {
    activePane = pane;
    terminalRef?.sendSelectPane(pane);
  }

  onMount(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    window.addEventListener("keydown", handleKeydown);

    const mq = window.matchMedia("(max-width: 768px)");
    isMobile = mq.matches;
    function onMqChange(e: MediaQueryListEvent) { isMobile = e.matches; }
    mq.addEventListener("change", onMqChange);

    return () => {
      clearInterval(interval);
      window.removeEventListener("keydown", handleKeydown);
      mq.removeEventListener("change", onMqChange);
    };
  });
</script>

<div class="flex h-screen bg-surface text-primary">
  <!-- Sidebar: fixed overlay on mobile, static on desktop -->
  {#if !isMobile || sidebarOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    {#if isMobile}
      <div
        class="fixed inset-0 bg-black/50 z-40"
        onclick={() => (sidebarOpen = false)}
        onkeydown={(e) => { if (e.key === "Escape") sidebarOpen = false; }}
      ></div>
    {/if}
    <aside class="{isMobile ? 'fixed inset-0 z-50 w-full' : 'w-[220px] min-w-[220px]'} bg-sidebar border-r border-edge flex flex-col overflow-hidden">
      <div class="flex items-center justify-between p-4 border-b border-edge">
        <h1 class="text-base font-semibold">Windmill</h1>
        <div class="flex items-center gap-2">
          <button
            class="h-8 px-2 gap-1.5 rounded-md border border-edge bg-surface text-accent text-xs flex items-center justify-center cursor-pointer hover:bg-hover disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={() => (showCreateDialog = true)}
            disabled={creating}
            title="New Worktree (Cmd+K)"
          ><span class="text-lg leading-none">+</span> New</button>
          {#if isMobile}
            <button
              class="h-8 w-8 rounded-md border border-edge bg-surface text-muted text-sm flex items-center justify-center cursor-pointer hover:bg-hover"
              onclick={() => (sidebarOpen = false)}
              title="Close sidebar"
            >&times;</button>
          {/if}
        </div>
      </div>
      <WorktreeList
        worktrees={visibleWorktrees}
        selected={selectedBranch}
        removing={removingBranches}
        onselect={(b) => { selectedBranch = b; if (isMobile) sidebarOpen = false; }}
        onremove={(b) => (removeBranch = b)}
      />
      {#if !isMobile}
        <div class="shrink-0 border-t border-edge px-4 py-3 text-[11px] text-muted flex flex-col gap-1">
          <div class="flex justify-between"><span>Navigate</span><kbd class="opacity-60">Cmd+Up/Down</kbd></div>
          <div class="flex justify-between"><span>New worktree</span><kbd class="opacity-60">Cmd+K</kbd></div>
          <div class="flex justify-between"><span>Merge</span><kbd class="opacity-60">Cmd+M</kbd></div>
          <div class="flex justify-between"><span>Remove</span><kbd class="opacity-60">Cmd+D</kbd></div>
        </div>
      {/if}
    </aside>
  {/if}

  <main class="flex-1 min-w-0 flex flex-col overflow-hidden">
    <TopBar
      name={selectedBranch}
      worktree={selectedWorktree}
      {sshHost}
      {isMobile}
      ontogglesidebar={() => (sidebarOpen = !sidebarOpen)}
      onmerge={() => { if (selectedBranch) mergeBranch = selectedBranch; }}
      onremove={() => { if (selectedBranch) removeBranch = selectedBranch; }}
      onsettings={() => (showSettingsDialog = true)}
    />

    {#if canConnect}
      {#key selectedBranch}
        <Terminal
          worktree={selectedBranch!}
          {isMobile}
          initialPane={isMobile ? activePane : undefined}
          bind:this={terminalRef}
        />
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

    {#if showPaneBar}
      <PaneBar {activePane} profile={paneBarProfile!} onselect={handlePaneSelect} />
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

{#if mergeBranch}
  <ConfirmDialog
    message={`Merge worktree "${mergeBranch}" into main? The worktree will be removed after merging.`}
    confirmLabel="Merge"
    variant="accent"
    loading={merging}
    error={mergeError}
    onconfirm={handleMerge}
    oncancel={() => { mergeBranch = null; mergeError = ""; }}
  />
{/if}

{#if showSettingsDialog}
  <SettingsDialog
    onsave={(host) => { sshHost = host; showSettingsDialog = false; }}
    onclose={() => (showSettingsDialog = false)}
  />
{/if}

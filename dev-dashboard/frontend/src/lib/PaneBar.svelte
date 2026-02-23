<script lang="ts">
  let { activePane, profile, onselect }: {
    activePane: number;
    profile: "full" | "agent-yolo";
    onselect: (pane: number) => void;
  } = $props();

  const panesByProfile = {
    "full": [
      { index: 0, label: "Claude" },
      { index: 1, label: "Backend" },
      { index: 2, label: "Frontend" },
    ],
    "agent-yolo": [
      { index: 0, label: "Claude" },
      { index: 1, label: "Shell" },
    ],
  };

  let panes = $derived(panesByProfile[profile] ?? panesByProfile["full"]);
</script>

<nav class="flex items-stretch bg-topbar border-t border-edge pane-bar">
  {#each panes as p (p.index)}
    <button
      type="button"
      class="flex-1 py-3 text-sm font-medium cursor-pointer border-none bg-transparent {activePane === p.index ? 'text-accent pane-active' : 'text-muted'}"
      onclick={() => onselect(p.index)}
    >
      {p.label}
    </button>
  {/each}
</nav>

<style>
  .pane-bar {
    padding-bottom: env(safe-area-inset-bottom, 0px);
  }
  .pane-active {
    box-shadow: inset 0 2px 0 0 var(--color-accent);
  }
</style>

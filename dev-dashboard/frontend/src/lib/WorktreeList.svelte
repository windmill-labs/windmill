<script lang="ts">
  import type { WorktreeInfo } from "./types";

  let { worktrees, selected, removing, onselect, onremove }: {
    worktrees: WorktreeInfo[];
    selected: string | null;
    removing: Set<string>;
    onselect: (branch: string) => void;
    onremove: (branch: string) => void;
  } = $props();

  function dotColor(agent: string): string {
    if (agent === "working") return "bg-success";
    if (agent === "waiting") return "bg-warning";
    if (agent === "error") return "bg-danger";
    return "bg-muted";
  }
</script>

<ul class="list-none overflow-y-auto flex-1 p-2">
  {#each worktrees as wt (wt.branch)}
    {@const isMain = wt.path === "(here)" || wt.branch === "main"}
    {@const isActive = wt.branch === selected}
    {@const isRemoving = removing.has(wt.branch)}
    <li class="mb-0.5 group relative {isRemoving ? 'opacity-40 pointer-events-none' : ''}">
      <button
        type="button"
        class="w-full py-2.5 px-3 rounded-md border cursor-pointer flex flex-col gap-1 text-left text-inherit text-sm bg-transparent hover:bg-hover {isActive ? 'bg-active border-accent' : 'border-transparent'}"
        onclick={() => onselect(wt.branch)}
      >
        <span class="font-medium truncate pr-5">{wt.branch}</span>
        <span class="flex gap-2 text-[11px] text-muted items-center flex-wrap">
          <span><span class="inline-block w-2 h-2 rounded-full mr-1 align-middle {dotColor(wt.agent)}"></span>{wt.agent || "none"}</span>
          {#if wt.agentName}
            <span>{wt.agentName}</span>
          {/if}
          {#if wt.profile}
            <span>{wt.profile}</span>
          {/if}
          {#if isMain}
            <span>main</span>
          {/if}
          {#if wt.backendPort}
            <span class="font-mono {wt.backendRunning ? 'text-success' : ''}">BE:{wt.backendPort}</span>
          {/if}
          {#if wt.frontendPort}
            <span class="font-mono {wt.frontendRunning ? 'text-success' : ''}">FE:{wt.frontendPort}</span>
          {/if}
        </span>
      </button>
      {#if !isMain}
        <button
          type="button"
          class="absolute top-2 right-2 w-5 h-5 rounded flex items-center justify-center text-muted hover:text-danger hover:bg-hover opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
          title="Remove worktree"
          onclick={(e) => { e.stopPropagation(); onremove(wt.branch); }}
        >&times;</button>
      {/if}
    </li>
  {/each}
</ul>

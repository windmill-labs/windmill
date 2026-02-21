<script lang="ts">
  import type { Profile } from "./api";

  const PROFILES: { value: Profile; label: string }[] = [
    { value: "agent-only", label: "Agent only" },
    { value: "agent-yolo", label: "Agent (skip permissions)" },
    { value: "full", label: "Full (agent + backend + frontend)" },
  ];

  let { loading = false, oncreate, oncancel }: {
    loading?: boolean;
    oncreate: (name: string, profile: Profile) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  let profile = $state<Profile>("agent-only");

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    dialogEl?.showModal();
  });

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<dialog bind:this={dialogEl} onclose={oncancel} class="bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[380px] w-[90%]">
  <form method="dialog" onsubmit={(e) => { e.preventDefault(); oncreate(name.trim(), profile); }}>
    <h2 class="text-base mb-4">New Worktree</h2>
    <div class="mb-4">
      <label class="block text-xs text-muted mb-1.5" for="wt-name">Name <span class="opacity-60">(optional)</span></label>
      <input
        id="wt-name"
        type="text"
        class="w-full px-2.5 py-1.5 rounded-md border border-edge bg-surface text-primary text-[13px] placeholder:text-muted/50 outline-none focus:border-accent"
        placeholder="auto-generated if empty"
        bind:value={name}
      />
    </div>
    <div class="flex flex-col gap-2 mb-6">
      {#each PROFILES as p}
        <label
          class="flex items-center gap-2.5 p-2.5 rounded-lg border cursor-pointer text-[13px] transition-colors
            {profile === p.value ? 'border-accent bg-accent/10' : 'border-edge hover:bg-hover'}"
        >
          <input
            type="radio"
            name="profile"
            value={p.value}
            checked={profile === p.value}
            onchange={() => (profile = p.value)}
            class="accent-[var(--accent)]"
          />
          {p.label}
        </label>
      {/each}
    </div>
    <div class="flex justify-end gap-2">
      <button type="button" class={btn} onclick={oncancel} disabled={loading}>Cancel</button>
      <button
        type="submit"
        class="{btn} !bg-accent !text-white !border-accent hover:!opacity-90 disabled:!opacity-50 disabled:!cursor-not-allowed flex items-center gap-1.5"
        disabled={loading}
      >{#if loading}<span class="spinner"></span>{/if} Create</button>
    </div>
  </form>
</dialog>

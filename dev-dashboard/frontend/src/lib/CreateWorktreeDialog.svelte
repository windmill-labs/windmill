<script lang="ts">
  import type { Profile, Agent } from "./api";

  const AGENTS: { value: Agent; label: string }[] = [
    { value: "claude", label: "Claude" },
    { value: "codex", label: "Codex" },
  ];

  const PROFILES: { value: Profile; label: string }[] = [
    { value: "full", label: "Full (agent + backend + frontend)" },
    { value: "agent-yolo", label: "Agent (sandboxed, yolo mode)" },
  ];

  let {
    loading = false,
    oncreate,
    oncancel,
  }: {
    loading?: boolean;
    oncreate: (name: string, profile: Profile, agent: Agent) => void;
    oncancel: () => void;
  } = $props();

  const STORAGE_KEY = "wt-default-profile";
  const AGENT_STORAGE_KEY = "wt-default-agent";
  const savedProfile = localStorage.getItem(STORAGE_KEY) as Profile | null;
  const savedAgent = localStorage.getItem(AGENT_STORAGE_KEY) as Agent | null;

  let name = $state("");
  let agent = $state<Agent>(savedAgent ?? "claude");
  let profile = $state<Profile>(savedProfile ?? "full");
  let saveDefault = $state(false);

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    dialogEl?.showModal();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      const idx = PROFILES.findIndex((p) => p.value === profile);
      const next = e.key === "ArrowDown"
        ? (idx + 1) % PROFILES.length
        : (idx - 1 + PROFILES.length) % PROFILES.length;
      profile = PROFILES[next].value;
    } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const idx = AGENTS.findIndex((a) => a.value === agent);
      const next = e.key === "ArrowRight"
        ? (idx + 1) % AGENTS.length
        : (idx - 1 + AGENTS.length) % AGENTS.length;
      agent = AGENTS[next].value;
    }
  }

  const btn =
    "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<dialog
  bind:this={dialogEl}
  onclose={oncancel}
  onkeydown={handleKeydown}
  class="bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[380px] w-[90%]"
>
  <form
    method="dialog"
    onsubmit={(e) => {
      e.preventDefault();
      if (saveDefault) {
        localStorage.setItem(STORAGE_KEY, profile);
        localStorage.setItem(AGENT_STORAGE_KEY, agent);
      } else {
        localStorage.removeItem(STORAGE_KEY);
        localStorage.removeItem(AGENT_STORAGE_KEY);
      }
      oncreate(name.trim(), profile, agent);
    }}
  >
    <h2 class="text-base mb-4">New Worktree</h2>
    <div class="mb-4">
      <label class="block text-xs text-muted mb-1.5" for="wt-name"
        >Name <span class="opacity-60">(optional)</span></label
      >
      <input
        id="wt-name"
        type="text"
        class="w-full px-2.5 py-1.5 rounded-md border border-edge bg-surface text-primary text-[13px] placeholder:text-muted/50 outline-none focus:border-accent"
        placeholder="auto-generated if empty"
        bind:value={name}
      />
    </div>
    <div class="flex gap-2 mb-4">
      {#each AGENTS as a}
        <label
          class="flex-1 flex items-center justify-center gap-2 p-2.5 rounded-lg border cursor-pointer text-[13px] transition-colors
            {agent === a.value
            ? 'border-accent bg-accent/10'
            : 'border-edge hover:bg-hover'}"
        >
          <input
            type="radio"
            name="agent"
            value={a.value}
            checked={agent === a.value}
            onchange={() => (agent = a.value)}
            class="accent-[var(--accent)]"
          />
          {a.label}
        </label>
      {/each}
    </div>
    <div class="flex flex-col gap-2 mb-6">
      {#each PROFILES as p}
        <label
          class="flex items-center gap-2.5 p-2.5 rounded-lg border cursor-pointer text-[13px] transition-colors
            {profile === p.value
            ? 'border-accent bg-accent/10'
            : 'border-edge hover:bg-hover'}"
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
    <label
      class="flex items-center gap-2 mb-4 text-[13px] text-muted cursor-pointer"
    >
      <input
        type="checkbox"
        bind:checked={saveDefault}
        class="accent-[var(--accent)]"
      />
      Save as default
    </label>
    <div class="flex justify-end gap-2">
      <button type="button" class={btn} onclick={oncancel} disabled={loading}
        >Cancel</button
      >
      <button
        type="submit"
        class="{btn} !bg-accent !text-white !border-accent hover:!opacity-90 disabled:!opacity-50 disabled:!cursor-not-allowed flex items-center gap-1.5"
        disabled={loading}
        >{#if loading}<span class="spinner"></span>{/if} Create</button
      >
    </div>
  </form>
</dialog>

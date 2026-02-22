<script lang="ts">
  const STORAGE_KEY = "wt-ssh-host";

  let { onsave, onclose }: {
    onsave: (sshHost: string) => void;
    onclose: () => void;
  } = $props();

  let sshHost = $state(localStorage.getItem(STORAGE_KEY) ?? "");
  let dialogEl: HTMLDialogElement;
  let inputEl: HTMLInputElement;

  $effect(() => {
    dialogEl?.showModal();
    inputEl?.focus();
  });

  function handleSave() {
    const trimmed = sshHost.trim();
    if (trimmed) {
      localStorage.setItem(STORAGE_KEY, trimmed);
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
    onsave(trimmed);
  }

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<dialog bind:this={dialogEl} onclose={onclose} class="bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[380px] w-[90%]">
  <form method="dialog" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
    <h2 class="text-base mb-4">Settings</h2>
    <div class="mb-4">
      <label class="block text-xs text-muted mb-1.5" for="ssh-host">
        SSH Host <span class="opacity-60">(for "Open in Cursor")</span>
      </label>
      <input
        bind:this={inputEl}
        id="ssh-host"
        type="text"
        class="w-full px-2.5 py-1.5 rounded-md border border-edge bg-surface text-primary text-[13px] placeholder:text-muted/50 outline-none focus:border-accent"
        placeholder="e.g. devbox or 10.0.0.5"
        bind:value={sshHost}
      />
      <p class="text-[11px] text-muted mt-1.5">
        Must match an entry in your local <code class="text-accent/80">~/.ssh/config</code>. Leave empty for local mode.
      </p>
    </div>
    <div class="flex justify-end gap-2">
      <button type="button" class={btn} onclick={onclose}>Cancel</button>
      <button
        type="submit"
        class="{btn} !bg-accent !text-white !border-accent hover:!opacity-90"
      >Save</button>
    </div>
  </form>
</dialog>

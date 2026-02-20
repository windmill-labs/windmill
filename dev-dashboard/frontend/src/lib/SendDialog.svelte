<script lang="ts">
  let { onsubmit, oncancel }: {
    onsubmit: (prompt: string) => void;
    oncancel: () => void;
  } = $props();

  let dialogEl: HTMLDialogElement;
  let prompt = $state("");

  $effect(() => {
    dialogEl?.showModal();
  });

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    const trimmed = prompt.trim();
    if (trimmed) onsubmit(trimmed);
  }

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<dialog bind:this={dialogEl} onclose={oncancel} class="bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[440px] w-[90%]">
  <form onsubmit={handleSubmit}>
    <h2 class="text-base mb-4">Send Prompt</h2>
    <label class="block text-[13px] text-muted mb-3">
      Prompt
      <textarea
        rows="4"
        required
        placeholder="Implement the feature..."
        bind:value={prompt}
        class="block w-full mt-1 p-2 bg-surface border border-edge rounded-md text-primary text-[13px]"
      ></textarea>
    </label>
    <div class="flex justify-end gap-2 mt-4">
      <button type="button" class={btn} onclick={oncancel}>Cancel</button>
      <button type="submit" class="{btn} !bg-accent !text-white !border-accent hover:!opacity-90">Send</button>
    </div>
  </form>
</dialog>

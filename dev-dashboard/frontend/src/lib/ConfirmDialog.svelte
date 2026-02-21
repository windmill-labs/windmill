<script lang="ts">
  let { message, loading = false, onconfirm, oncancel }: {
    message: string;
    loading?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    dialogEl?.showModal();
  });

  const btn = "px-3 py-1.5 rounded-md border border-edge bg-surface text-primary text-xs cursor-pointer hover:bg-hover";
</script>

<dialog bind:this={dialogEl} onclose={oncancel} class="bg-sidebar text-primary border border-edge rounded-xl p-6 max-w-[380px] w-[90%]">
  <h2 class="text-base mb-4">Confirm</h2>
  <p class="text-[13px] text-muted mb-6">{message}</p>
  <div class="flex justify-end gap-2">
    <button type="button" class={btn} onclick={oncancel} disabled={loading}>Cancel</button>
    <button
      type="button"
      class="{btn} !bg-danger !text-white !border-danger hover:!opacity-90 disabled:!opacity-50 disabled:!cursor-not-allowed flex items-center gap-1.5"
      onclick={onconfirm}
      disabled={loading}
    >{#if loading}<span class="spinner"></span>{/if} Remove</button>
  </div>
</dialog>

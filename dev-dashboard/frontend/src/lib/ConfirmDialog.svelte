<script lang="ts">
  let { message, loading = false, error = "", confirmLabel = "Remove", variant = "danger", onconfirm, oncancel }: {
    message: string;
    loading?: boolean;
    error?: string;
    confirmLabel?: string;
    variant?: "danger" | "accent";
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
  <form method="dialog" onsubmit={(e) => { e.preventDefault(); onconfirm(); }}>
    <h2 class="text-base mb-4">Confirm</h2>
    <p class="text-[13px] text-muted mb-6">{message}</p>
    {#if error}<p class="text-[12px] text-danger mb-4 -mt-2 whitespace-pre-wrap">{error}</p>{/if}
    <div class="flex justify-end gap-2">
      <button type="button" class={btn} onclick={oncancel} disabled={loading}>Cancel</button>
      <button
        type="submit"
        class="{btn} !text-white hover:!opacity-90 disabled:!opacity-50 disabled:!cursor-not-allowed flex items-center gap-1.5 {variant === 'accent' ? '!bg-accent !border-accent' : '!bg-danger !border-danger'}"
        disabled={loading}
      >{#if loading}<span class="spinner"></span>{/if} {confirmLabel}</button>
    </div>
  </form>
</dialog>

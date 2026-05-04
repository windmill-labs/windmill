<script lang="ts">
  let {
    label = $bindable('Admin'),
    expires_at = $bindable(new Date()),
    is_valid = $bindable(true)
  }: {
    label?: string
    expires_at?: Date
    is_valid?: boolean
  } = $props()
</script>

<div class="admin-token-badge">
  <span class="label">{label}</span>
  <span class="expires">expires {expires_at.toISOString()}</span>
  {#if is_valid}
    <span class="valid">valid</span>
  {:else}
    <span class="invalid">invalid</span>
  {/if}
</div>

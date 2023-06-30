<script lang="ts">
	import Portal from 'svelte-portal'

	export let condition = false
</script>

{#if condition}
	<Portal><slot /></Portal>
{:else}
	<slot />
{/if}

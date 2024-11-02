<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	export let condition = false
</script>

{#if condition}
	<Portal name="conditional-portal-global"><slot /></Portal>
{:else}
	<slot />
{/if}

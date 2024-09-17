<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	export let condition = false
	export let target: string | HTMLElement | undefined = undefined
</script>

{#if condition}
	<Portal {target}><slot /></Portal>
{:else}
	<slot />
{/if}

<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	export let condition = false
	export let target: string | HTMLElement | undefined = undefined
</script>

{#if condition}
	<Portal name="conditional-portal" {target}><slot /></Portal>
{:else}
	<slot />
{/if}

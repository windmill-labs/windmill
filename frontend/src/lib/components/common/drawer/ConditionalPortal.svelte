<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	interface Props {
		condition?: boolean
		target?: string | HTMLElement | undefined
		name?: string
		children?: import('svelte').Snippet
	}

	let {
		condition = false,
		target = undefined,
		name = 'conditional-portal',
		children
	}: Props = $props()
</script>

{#if condition}
	<Portal {name} {target}>{@render children?.()}</Portal>
{:else}
	{@render children?.()}
{/if}

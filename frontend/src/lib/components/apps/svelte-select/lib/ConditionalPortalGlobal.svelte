<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	interface Props {
		condition?: boolean
		children?: import('svelte').Snippet
	}

	let { condition = false, children }: Props = $props()
</script>

{#if condition}
	<Portal name="conditional-portal-global">{@render children?.()}</Portal>
{:else}
	{@render children?.()}
{/if}

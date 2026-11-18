<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	interface Props {
		condition?: boolean
		target?: string | HTMLElement | undefined
		name?: string
		class?: string | undefined
		children?: import('svelte').Snippet
	}

	let {
		condition = false,
		target = undefined,
		name = 'conditional-portal',
		class: clazz = undefined,
		children
	}: Props = $props()
</script>

{#if condition}
	<Portal {name} {target} class={clazz}>{@render children?.()}</Portal>
{:else}
	{@render children?.()}
{/if}

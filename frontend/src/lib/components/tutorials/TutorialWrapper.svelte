<script lang="ts">
	import { getTutorialIndex } from '$lib/tutorials/config'

	interface Props {
		id: string
		component: any // Svelte component type - using any to avoid complex type issues
		name?: string
		onInstanceReady: (id: string, instance: any) => void
		onSkipAll: () => void
		onerror?: (...args: any[]) => any
		onreload?: (...args: any[]) => any
	}

	let { id, component: Component, name, onInstanceReady, onSkipAll,
		onerror = undefined,
		onreload = undefined }: Props = $props()

	let instance: any = $state(undefined)
	const index = getTutorialIndex(id)

	$effect(() => {
		if (instance) {
			onInstanceReady(id, instance)
		}
	})
</script>

{#if Component}
	{@const Comp = Component}
	<Comp
		bind:this={instance}
		{index}
		{...(name ? { name } : {})}
		onerror={onerror}
		onskipAll={onSkipAll}
		onreload={onreload}
	/>
{/if}


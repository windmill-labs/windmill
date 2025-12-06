<script lang="ts">
	import { getTutorialIndex } from '$lib/tutorials/config'

	interface Props {
		id: string
		component: any // Svelte component type - using any to avoid complex type issues
		name?: string
		onInstanceReady: (id: string, instance: any) => void
		onSkipAll: () => void
	}

	let { id, component: Component, name, onInstanceReady, onSkipAll }: Props = $props()

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
		on:error
		on:skipAll={onSkipAll}
		on:reload
	/>
{/if}


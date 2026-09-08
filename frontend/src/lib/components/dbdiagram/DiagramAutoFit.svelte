<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte'
	import { tick } from 'svelte'

	interface Props {
		/** Refits whenever this changes — the set of drawn tables, not their
		 * heights, so expanding a card doesn't move the view under the pointer. */
		key: string
	}

	let { key }: Props = $props()

	// Its own component so the hook runs inside the flow, as GraphZoomControls does.
	const { fitView } = useSvelteFlow()

	$effect(() => {
		key
		tick().then(() => fitView({ maxZoom: 1 }))
	})
</script>

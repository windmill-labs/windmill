<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte'
	import { tick, untrack } from 'svelte'

	interface Props {
		nodes: any[]
		triggerCount: number
	}

	let { nodes, triggerCount }: Props = $props()
	const { fitView } = useSvelteFlow()

	let hasRunInitialFit = false

	/**
	 * Smart fitView that fits all nodes and notes with minimal zoom adjustment
	 * Allows zoom to change by ±0.2 to fit content if needed
	 * Only runs once after the first flow build
	 */
	async function smartFitView() {
		if (hasRunInitialFit || nodes.length === 0) {
			return
		}

		await tick()

		try {
			fitView({
				minZoom: Math.max(1),
				maxZoom: Math.min(1),
				padding: 0.2,
				duration: 0
			})

			hasRunInitialFit = true
		} catch (e) {
			console.debug('smartFitView error:', e)
		}
	}

	// Run when nodes are ready and trigger count changes (after first build)
	$effect(() => {
		triggerCount
		untrack(() => {
			if (!hasRunInitialFit && nodes.length > 0) {
				smartFitView()
			}
		})
	})
</script>

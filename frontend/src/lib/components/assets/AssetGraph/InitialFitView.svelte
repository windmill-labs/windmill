<script lang="ts">
	import { useNodesInitialized, useSvelteFlow, type Node } from '@xyflow/svelte'

	// One-shot initial fit of the viewport to the laid-out graph. The graph
	// arrives async after the canvas mounts (the first render often holds only
	// the + node), so SvelteFlow's own init-time `fitView` prop would fit the
	// wrong content — instead wait for the first real node set to be measured.
	// `maxZoom: 1` means small graphs are centered at natural scale and only
	// oversized ones zoom out. Lives inside <SvelteFlow> so useSvelteFlow()
	// has the flow context (same pattern as PanToNode).
	//
	// `fitKey` re-arms the fit when it changes: the in-app folder switcher
	// swaps the whole graph without remounting the canvas, and the new
	// pipeline deserves its own initial fit.
	let { nodes, fitKey = '' }: { nodes: Node[]; fitKey?: string } = $props()

	const { fitView } = useSvelteFlow()
	const nodesInitialized = useNodesInitialized()

	let fittedFor: string | undefined = $state(undefined)
	$effect(() => {
		if (fittedFor === fitKey) return
		// useNodesInitialized flips false while any node is unmeasured, so by
		// the time it's true again the fresh nodes have real dimensions.
		if (!nodesInitialized.current) return
		if (!nodes.some((n) => n.type !== 'add')) return
		fittedFor = fitKey
		void fitView({ padding: 0.1, maxZoom: 1 })
	})
</script>

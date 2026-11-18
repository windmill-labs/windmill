<script lang="ts">
	import { ControlButton, useSvelteFlow, type Node } from '@xyflow/svelte'
	import { Maximize, ZoomIn, ZoomOut } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { untrack } from 'svelte'

	interface Props {
		/** Nodes to frame; notes are decorative and would skew the fit. */
		fitViewNodes?: Node[]
		/** Frame the whole graph when it appears, resizes or gains nodes. A pan or zoom by the
		 * viewer is left alone until then. Never zooms past 100%, so small graphs keep their size. */
		fitOnLoad?: boolean
		/** Canvas size, tracked so `fitOnLoad` refits once it is measured. */
		canvasWidth?: number
		canvasHeight?: number
	}

	let { fitViewNodes, fitOnLoad = false, canvasWidth, canvasHeight }: Props = $props()

	// Its own component so the hook runs inside the provider, like DragGhost and
	// PaneContextMenu do.
	const { zoomIn, zoomOut, fitView } = useSvelteFlow()

	const nodeCount = $derived(fitViewNodes?.length ?? 0)

	$effect(() => {
		if (!fitOnLoad || !nodeCount || !canvasWidth || !canvasHeight) return
		// Nodes must be measured by the flow before they can be framed.
		const frame = requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				fitView({ nodes: untrack(() => fitViewNodes), maxZoom: 1 })
			})
		)
		return () => cancelAnimationFrame(frame)
	})
</script>

<Tooltip>
	<ControlButton onclick={() => zoomIn()}>
		<ZoomIn size="14" />
	</ControlButton>
	{#snippet text()}Zoom in{/snippet}
</Tooltip>
<Tooltip>
	<ControlButton onclick={() => zoomOut()}>
		<ZoomOut size="14" />
	</ControlButton>
	{#snippet text()}Zoom out{/snippet}
</Tooltip>
<Tooltip>
	<ControlButton onclick={() => fitView(fitViewNodes ? { nodes: fitViewNodes } : undefined)}>
		<Maximize size="14" />
	</ControlButton>
	{#snippet text()}Fit view{/snippet}
</Tooltip>

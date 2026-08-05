<script lang="ts">
	import { ControlButton, useSvelteFlow, type Node } from '@xyflow/svelte'
	import { Maximize, ZoomIn, ZoomOut } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'

	interface Props {
		/** Nodes to frame; notes are decorative and would skew the fit. */
		fitViewNodes?: Node[]
	}

	let { fitViewNodes }: Props = $props()

	// Its own component so the hook runs inside the provider, like DragGhost and
	// PaneContextMenu do.
	const { zoomIn, zoomOut, fitView } = useSvelteFlow()
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

<script lang="ts">
	import { SvelteFlow, SvelteFlowProvider, Controls } from '@xyflow/svelte'
	import '@xyflow/svelte/dist/base.css'
	import type { FlowModule } from '$lib/gen'
	import type { Snippet } from 'svelte'
	import { layoutSimplifiedFlow } from './simplifiedLayout'
	import SimplifiedNode from './SimplifiedNode.svelte'
	import SimplifiedBox from './SimplifiedBox.svelte'
	import SimplifiedEdge from './SimplifiedEdge.svelte'

	interface Props {
		modules: FlowModule[] | undefined
		failureModule?: FlowModule
		preprocessorModule?: FlowModule
		/** The view sizes itself to its content, within these bounds */
		minHeight?: number
		maxHeight?: number
		/** Called with a module id, or `Input` / `Result` / `failure` / `preprocessor` */
		onSelect?: (nodeId: string) => void
		/** Rendered in the top-left control stack, e.g. the toggle leading back to the full graph */
		topLeftControls?: Snippet
	}

	let {
		modules,
		failureModule,
		preprocessorModule,
		minHeight = 0,
		maxHeight,
		onSelect,
		topLeftControls
	}: Props = $props()

	let layout = $derived(
		layoutSimplifiedFlow({ modules: modules ?? [], failureModule, preprocessorModule })
	)

	const PAD = 72
	const MIN_READABLE_ZOOM = 0.7

	let bounds = $derived.by(() => {
		const ns = layout.nodes
		const x0 = Math.min(0, ...ns.map((n) => n.position.x))
		const y0 = Math.min(0, ...ns.map((n) => n.position.y))
		const x1 = Math.max(0, ...ns.map((n) => n.position.x + (n.width ?? 0)))
		const y1 = Math.max(0, ...ns.map((n) => n.position.y + (n.height ?? 0)))
		return { x0, y0, width: x1 - x0, height: y1 - y0 }
	})

	// Room for the graph at zoom 1 plus the controls overlaid on top.
	let height = $derived.by(() => {
		const content = bounds.height + 2 * PAD
		return Math.max(minHeight, Math.min(content, maxHeight ?? content))
	})

	let width = $state(0)

	// A long flow fitted to the width would shrink to unreadable nodes, so past a point it opens at a
	// readable zoom anchored on its start and is panned horizontally instead.
	function initialViewport() {
		const fit = Math.min(1, (width - 48) / bounds.width, (height - 48) / bounds.height)
		const zoom = Math.max(fit, Math.min(MIN_READABLE_ZOOM, (height - 48) / bounds.height))
		const x =
			fit === zoom ? (width - bounds.width * zoom) / 2 - bounds.x0 * zoom : 24 - bounds.x0 * zoom
		const y = (height - bounds.height * zoom) / 2 - bounds.y0 * zoom
		return { x, y, zoom }
	}

	const nodeTypes = { simplifiedNode: SimplifiedNode, simplifiedBox: SimplifiedBox } as any
	const edgeTypes = { simplifiedEdge: SimplifiedEdge } as any
	const proOptions = { hideAttribution: true }
</script>

<div class="w-full" style="height: {height}px;" bind:clientWidth={width}>
	{#if width > 0}
		<SvelteFlowProvider>
			<SvelteFlow
				nodes={layout.nodes}
				edges={layout.edges}
				{nodeTypes}
				{edgeTypes}
				initialViewport={initialViewport()}
				minZoom={0.2}
				maxZoom={1.6}
				nodesDraggable={false}
				nodesConnectable={false}
				elementsSelectable={true}
				zoomOnDoubleClick={false}
				deleteKey={null}
				onnodeclick={({ node }) => {
					// Boxes stand for their loop or branch step; placeholders are not selectable
					if (node.type === 'simplifiedBox' || node.selectable) onSelect?.(node.id)
				}}
				{proOptions}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full"></div>
				<Controls position="top-right" orientation="horizontal" showLock={false} />
				{#if topLeftControls}
					<Controls
						position="top-left"
						orientation="vertical"
						showLock={false}
						showZoom={false}
						showFitView={false}
						class="!shadow-none gap-3"
					>
						{@render topLeftControls()}
					</Controls>
				{/if}
			</SvelteFlow>
		</SvelteFlowProvider>
	{/if}
</div>

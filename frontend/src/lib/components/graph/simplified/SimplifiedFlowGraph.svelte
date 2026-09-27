<script lang="ts">
	import {
		SvelteFlow,
		SvelteFlowProvider,
		Controls,
		SelectionMode,
		type Node
	} from '@xyflow/svelte'
	import '@xyflow/svelte/dist/base.css'
	import type { FlowModule } from '$lib/gen'
	import type { Snippet } from 'svelte'
	import { layoutSimplifiedFlow } from './simplifiedLayout'
	import { setSimplifiedEditContext } from './simplifiedContext'
	import type { FlowStructureNode } from '../flowStructure'
	import type { GraphEventHandlers } from '../graphBuilder.svelte'
	import type { SelectionManager } from '../selectionUtils.svelte'
	import SelectionTool from '../SelectionTool.svelte'
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
		/** The graph's selection, owned by the caller so it survives switching views */
		selectedId?: string
		/** Rendered in the top-left control stack, e.g. the toggle leading back to the full graph */
		topLeftControls?: Snippet
		/** The step tree with groups resolved, as the full graph builds it */
		structure?: FlowStructureNode[]
		/** Enables the step menus */
		editMode?: boolean
		/** Enables the "+" slots, on top of `editMode` */
		insertable?: boolean
		eventHandlers?: GraphEventHandlers
		disableAi?: boolean
		/** The graph's selection manager; with `selectionOverlay`, several steps can be selected */
		selectionManager?: SelectionManager
		/** Drawn over a multi-step selection (its bulk actions), given the laid-out nodes */
		selectionOverlay?: Snippet<[Node[]]>
		/** Held while dragging on the canvas to select a rectangle of steps */
		selectionKey?: string
	}

	let {
		modules,
		failureModule,
		preprocessorModule,
		minHeight = 0,
		maxHeight,
		onSelect,
		selectedId,
		topLeftControls,
		structure,
		editMode = false,
		insertable = false,
		eventHandlers,
		disableAi = false,
		selectionManager,
		selectionOverlay,
		selectionKey
	}: Props = $props()

	setSimplifiedEditContext({
		get eventHandlers() {
			return eventHandlers
		},
		get editMode() {
			return editMode
		},
		get disableAi() {
			return disableAi
		}
	})

	let layout = $derived(
		layoutSimplifiedFlow({
			modules: modules ?? [],
			structure,
			failureModule,
			preprocessorModule,
			editable: editMode && insertable && !!eventHandlers
		})
	)
	// The full graph selects a branch lane as `<module id>-branch-<n|default>`; here the lane
	// belongs to its group, which is keyed by the module id.
	let nodes = $derived(
		layout.nodes.map((n) => ({
			...n,
			// The selection box measures nodes by their `measured` size
			measured: { width: n.width, height: n.height },
			selected:
				n.id === selectedId ||
				!!selectionManager?.selectedIds.includes(n.id) ||
				(n.type === 'simplifiedBox' && !!selectedId?.startsWith(`${n.id}-branch-`))
		}))
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
				{nodes}
				edges={layout.edges}
				{nodeTypes}
				{edgeTypes}
				initialViewport={initialViewport()}
				minZoom={0.2}
				maxZoom={1.6}
				nodesDraggable={false}
				nodesConnectable={false}
				elementsSelectable={!!selectionOverlay}
				multiSelectionKey="Shift"
				selectionKey={selectionOverlay ? (selectionKey ?? null) : null}
				selectionMode={SelectionMode.Partial}
				zoomOnDoubleClick={false}
				deleteKey={null}
				onnodeclick={({ node, event }) => {
					// Adding to a multi-selection is xyflow's, synced by SelectionTool
					if (event.shiftKey) return
					// Boxes stand for their loop or branch step; placeholders are not selectable
					if (node.type === 'simplifiedBox' || node.selectable) onSelect?.(node.id)
				}}
				onpaneclick={() => selectionManager?.clearSelection()}
				{proOptions}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full"></div>
				{#if selectionOverlay && selectionManager}
					<SelectionTool {selectionManager} />
					{@render selectionOverlay(nodes)}
				{/if}
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

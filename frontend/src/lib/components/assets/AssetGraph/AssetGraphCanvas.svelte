<script lang="ts">
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		Controls,
		MiniMap,
		ConnectionLineType,
		type Node,
		type Edge,
		MarkerType
	} from '@xyflow/svelte'
	import AssetNode from './AssetNode.svelte'
	import RunnableNode from './RunnableNode.svelte'
	import TriggerNode, { type TriggerNodeKind } from './TriggerNode.svelte'
	import AddNode from './AddNode.svelte'
	import AssetGraphEdge from './AssetGraphEdge.svelte'
	import { layoutAssetGraph } from './assetGraphLayout'
	import type { AssetGraphResponse, AssetGraphSelection } from './types'
	import type { AssetKind } from '$lib/gen'
	import { NODE } from '$lib/components/graph/util'

	// Width of the + node's rendered DOM element. Sugiyama allocates a full
	// NODE.width slot for every node, so the small round button ends up
	// left-aligned in its slot. We compensate by shifting the + node right
	// by half the difference so its visual center matches the slot center.
	const ADD_NODE_WIDTH = 40

	interface Props {
		graph: AssetGraphResponse
		selection?: AssetGraphSelection | undefined
		onselect?: (selection: AssetGraphSelection | undefined) => void
		// Called when the user clicks the per-asset + button (consumer-script
		// entry). Kept optional so the canvas stays usable outside the
		// pipeline editor.
		onAddScriptForAsset?: (
			asset: { kind: AssetKind; path: string },
			language: import('$lib/gen').ScriptLang,
			scriptPath: string
		) => void
		// Pipeline-wide + node shown at the top of the graph. Picking any
		// kind from the menu invokes this one callback with the chosen
		// trigger source — the page uses it to seed the draft's annotation.
		onAddMaterializer?: (
			language: import('$lib/gen').ScriptLang,
			path: string,
			source:
				| { kind: 'schedule'; cron: string }
				| {
						kind: 'webhook' | 'email' | 'kafka' | 'mqtt' | 'nats' | 'postgres' | 'sqs' | 'gcp'
						path: string | undefined
				  }
		) => void
		// Folder-scoped prefix shown as a read-only chip in the insert menu
		// path input (e.g. `f/{folder}/`). Shared across top + and per-asset +.
		pathPrefix?: string
		// Seeded editable suffix (e.g. `new_materializer`).
		defaultPathSuffix?: string
		// Default cron expression seeded into the top + template.
		defaultScheduleCron?: string
	}
	let {
		graph,
		selection,
		onselect,
		onAddScriptForAsset,
		onAddMaterializer,
		pathPrefix = '',
		defaultPathSuffix = '',
		defaultScheduleCron = ''
	}: Props = $props()

	const ADD_NODE_ID = '__add__'

	type BuiltEdge = {
		id: string
		source: string
		target: string
		// 'add-anchor' edges connect the + node to every otherwise-root node in
		// the DAG. They force sugiyama to put + at layer 0 (top) and center
		// it horizontally over the roots — same mechanism the flow editor
		// uses for its Trigger node. Filtered out of rendered edges.
		kind:
			| 'lineage-write'
			| 'lineage-read'
			| 'trigger-asset'
			| 'trigger-schedule'
			| 'trigger-native'
			| 'add-anchor'
		unsaved?: boolean
	}

	// Lineage edges (parsed r/w usages): writer → asset, asset → reader.
	// Trigger edges (`// on <x>`): asset → script or schedule → script. The
	// lineage subgraph is informational; the trigger subgraph is executable.
	function build(g: AssetGraphResponse) {
		const nodes: Array<{
			id: string
			type: 'asset' | 'runnable' | 'trigger' | 'add'
			data: any
		}> = []
		const edges: BuiltEdge[] = []

		const hasAddNode = onAddMaterializer != null
		if (hasAddNode) {
			nodes.push({
				id: ADD_NODE_ID,
				type: 'add',
				data: {
					onAddMaterializer: onAddMaterializer!,
					pathPrefix,
					defaultPathSuffix,
					defaultScheduleCron
				}
			})
		}

		for (const a of g.assets) {
			const assetId = `asset:${a.kind}:${a.path}`
			nodes.push({
				id: assetId,
				type: 'asset',
				data: {
					asset_kind: a.kind,
					path: a.path,
					onAddScript: onAddScriptForAsset,
					pathPrefix,
					defaultPathSuffix
				}
			})
		}
		for (const r of g.runnables) {
			nodes.push({
				id: `${r.usage_kind}:${r.path}`,
				type: 'runnable',
				data: {
					runnable_kind: r.usage_kind,
					path: r.path,
					is_materializer: r.is_materializer ?? false
				}
			})
		}

		for (const e of g.edges) {
			const runnableId = `${e.runnable_kind}:${e.runnable_path}`
			const assetId = `asset:${e.asset_kind}:${e.asset_path}`
			const access = e.access_type ?? 'r'
			if (access === 'w' || access === 'rw') {
				edges.push({
					id: `prod:${runnableId}->${assetId}`,
					source: runnableId,
					target: assetId,
					kind: 'lineage-write',
					unsaved: e.unsaved
				})
			}
			if (access === 'r' || access === 'rw') {
				edges.push({
					id: `cons:${assetId}->${runnableId}`,
					source: assetId,
					target: runnableId,
					kind: 'lineage-read',
					unsaved: e.unsaved
				})
			}
		}

		// Non-asset triggers (schedule + native) are rendered as source nodes
		// above the materializer. Nodes are deduplicated per (kind, ref)
		// tuple so a single schedule/webhook shared across multiple scripts
		// shows as one node with N outgoing edges. A trigger node is
		// considered unsaved if every attachment referencing it is unsaved.
		const triggerSourceNodes = new Map<
			string,
			{ allUnsaved: boolean; kind: TriggerNodeKind; ref: string }
		>()
		function recordSourceTrigger(id: string, kind: TriggerNodeKind, ref: string, unsaved: boolean) {
			const prev = triggerSourceNodes.get(id)
			if (!prev) {
				triggerSourceNodes.set(id, { allUnsaved: unsaved, kind, ref })
			} else {
				prev.allUnsaved = prev.allUnsaved && unsaved
			}
		}

		for (const t of g.triggers ?? []) {
			const runnableId = `${t.runnable_kind}:${t.runnable_path}`
			if (t.trigger_kind === 'asset') {
				const assetId = `asset:${t.asset_kind}:${t.asset_path}`
				edges.push({
					id: `trig-a:${assetId}->${runnableId}`,
					source: assetId,
					target: runnableId,
					kind: 'trigger-asset',
					unsaved: t.unsaved
				})
				continue
			}
			const ref = t.trigger_kind === 'schedule' ? (t as any).cron : (t as any).path
			const sourceId = `trigger:${t.trigger_kind}:${ref}`
			recordSourceTrigger(sourceId, t.trigger_kind, ref, !!t.unsaved)
			edges.push({
				id: `trig-${t.trigger_kind}:${sourceId}->${runnableId}`,
				source: sourceId,
				target: runnableId,
				kind: t.trigger_kind === 'schedule' ? 'trigger-schedule' : 'trigger-native',
				unsaved: t.unsaved
			})
		}
		for (const [id, info] of triggerSourceNodes) {
			nodes.push({
				id,
				type: 'trigger',
				data: { kind: info.kind, ref: info.ref, unsaved: info.allUnsaved }
			})
		}

		// Anchor the + to layer 0 by making it a parent of every node that
		// would otherwise be a root (no incoming edge), *including* schedule
		// nodes. Must run AFTER all real edges are added so we catch the
		// current set of roots. Sugiyama then places + on layer 0 and
		// centers it over whatever's below.
		if (hasAddNode) {
			const hasIncoming = new Set<string>()
			for (const e of edges) hasIncoming.add(e.target)
			for (const n of nodes) {
				if (n.id === ADD_NODE_ID) continue
				if (!hasIncoming.has(n.id)) {
					edges.push({
						id: `add-anchor:${n.id}`,
						source: ADD_NODE_ID,
						target: n.id,
						kind: 'add-anchor'
					})
				}
			}
		}

		return { nodes, edges }
	}

	let model = $derived(build(graph))

	let selectedId = $derived.by(() => {
		if (!selection) return undefined
		return selection.kind === 'asset'
			? `asset:${selection.asset_kind}:${selection.path}`
			: `${selection.runnable_kind}:${selection.path}`
	})

	// Pane width drives horizontal centering (same pattern FlowGraphV2 uses).
	// Bound on the outer wrapper; updates on pane resize via $state.
	let paneWidth = $state(800)

	let positionedNodes = $derived.by(() => {
		const positions = layoutAssetGraph({
			nodes: model.nodes.map((n) => ({ id: n.id, data: n.data })),
			edges: model.edges.map((e) => ({ source: e.source, target: e.target }))
		})
		// Compute bbox width from layout; shift every x so the graph is
		// horizontally centered inside the pane. y is untouched so layer 0
		// sits at the top of the viewport (matches flow editor's Trigger
		// placement — no fitView reshuffling).
		let minX = Infinity
		let maxX = -Infinity
		for (const p of positions.values()) {
			if (p.x < minX) minX = p.x
			if (p.x > maxX) maxX = p.x
		}
		const bboxWidth = isFinite(minX) ? maxX - minX : 0
		const xCenter = paneWidth / 2 - bboxWidth / 2
		return model.nodes.map<Node>((n) => {
			const p = positions.get(n.id) ?? { x: 0, y: 0 }
			// Compensate for the + node being narrower than its layout slot
			// so it visually centers over the node(s) below.
			const xShift = n.id === ADD_NODE_ID ? (NODE.width - ADD_NODE_WIDTH) / 2 : 0
			return {
				id: n.id,
				type: n.type,
				position: { x: p.x + xCenter + xShift, y: p.y + 40 },
				data: n.data,
				selected: n.id === selectedId,
				// All nodes non-draggable: the layout is sugiyama-computed,
				// dragging would fight the reactive re-layout. Selection is
				// still allowed on asset/runnable (not on the + or schedules)
				// for the details-pane click-through.
				draggable: false,
				selectable: n.id !== ADD_NODE_ID && n.type !== 'trigger'
			}
		})
	})

	let flowEdges = $derived.by(() =>
		model.edges
			// Anchor edges are layout-only.
			.filter((e) => e.kind !== 'add-anchor')
			.map<Edge>((e) => {
				let style: string
				let animated = false
				let markerColor: string | undefined = undefined
				let strokeDasharray: string | undefined = undefined
				let label: string | undefined = undefined
				let labelStyle: string | undefined = undefined
				switch (e.kind) {
					case 'lineage-write':
						style = 'stroke: rgb(59 130 246); stroke-width: 1.5px;'
						break
					case 'lineage-read':
						style = 'stroke: rgb(107 114 128); stroke-width: 1.25px;'
						break
					case 'trigger-asset':
						style = 'stroke: rgb(16 185 129); stroke-width: 2px;'
						animated = true
						markerColor = 'rgb(16 185 129)'
						label = 'triggers'
						labelStyle = 'fill: rgb(16 185 129); font-size: 10px; font-weight: 600;'
						break
					case 'trigger-schedule':
						style = 'stroke: rgb(245 158 11); stroke-width: 2px;'
						strokeDasharray = '6 3'
						markerColor = 'rgb(245 158 11)'
						label = 'schedule'
						labelStyle = 'fill: rgb(245 158 11); font-size: 10px; font-weight: 600;'
						break
					case 'trigger-native':
						// Colour neutral here because the trigger source node
						// already carries per-kind colour; edge just needs to
						// say "this fires it" distinctly from lineage.
						style = 'stroke: rgb(100 116 139); stroke-width: 2px;'
						strokeDasharray = '6 3'
						markerColor = 'rgb(100 116 139)'
						label = 'triggers'
						labelStyle = 'fill: rgb(100 116 139); font-size: 10px; font-weight: 600;'
						break
					default:
						style = ''
				}
				// Unsaved edges (trigger or lineage) get a distinct dashed
				// pattern + dimmed opacity so the user sees they're live-
				// parsed, not persisted. Lineage edges from base graph data
				// don't carry `unsaved`, only those synthesized by the draft
				// overlay (e.g. the random output asset).
				if (e.unsaved) {
					strokeDasharray = '3 3'
					style = `${style} opacity: 0.7;`
					animated = false
					if (label) label = `${label} (unsaved)`
				}
				if (strokeDasharray) {
					style = `${style} stroke-dasharray: ${strokeDasharray};`
				}
				return {
					id: e.id,
					source: e.source,
					target: e.target,
					type: 'asset',
					animated,
					label,
					labelStyle,
					labelBgStyle: label ? 'fill: rgb(255 255 255 / 0.9);' : undefined,
					style,
					markerEnd: {
						type: MarkerType.ArrowClosed,
						width: 14,
						height: 14,
						color: markerColor
					}
				}
			})
	)

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])
	$effect(() => {
		nodes = positionedNodes
	})
	$effect(() => {
		edges = flowEdges
	})

	const nodeTypes = {
		asset: AssetNode as any,
		runnable: RunnableNode as any,
		trigger: TriggerNode as any,
		add: AddNode as any
	}

	const edgeTypes = {
		asset: AssetGraphEdge as any
	}

	function handleNodeClick({ node }: { node: Node }) {
		if (!onselect) return
		const data = node.data as any
		if (node.type === 'asset') {
			onselect({ kind: 'asset', asset_kind: data.asset_kind, path: data.path })
		} else if (node.type === 'runnable') {
			onselect({ kind: 'runnable', runnable_kind: data.runnable_kind, path: data.path })
		}
		// 'schedule' doesn't produce a selection.
	}
</script>

<div class="w-full h-full relative" bind:clientWidth={paneWidth}>
	<SvelteFlow
		{nodes}
		{edges}
		{nodeTypes}
		{edgeTypes}
		minZoom={0.2}
		maxZoom={1.6}
		nodesDraggable={false}
		nodesConnectable={false}
		elementsSelectable
		zoomOnDoubleClick={false}
		connectionLineType={ConnectionLineType.SmoothStep}
		defaultEdgeOptions={{ type: 'asset' }}
		proOptions={{ hideAttribution: true }}
		onnodeclick={handleNodeClick}
		onpaneclick={() => onselect?.(undefined)}
		--background-color={false}
	>
		<div class="absolute inset-0 !bg-surface-secondary h-full"></div>
		<Controls />
		<MiniMap
			pannable
			zoomable
			class="!bg-surface"
			nodeColor={(n) =>
				n.type === 'asset'
					? 'rgb(96 165 250 / 0.5)'
					: n.type === 'trigger'
						? 'rgb(251 191 36 / 0.5)'
						: 'rgb(52 211 153 / 0.5)'}
			nodeStrokeColor="transparent"
			maskColor="rgb(0 0 0 / 0.2)"
		/>
	</SvelteFlow>
</div>

<style lang="postcss">
	:global(.svelte-flow__handle) {
		opacity: 0;
	}
	:global(.svelte-flow__controls-button) {
		@apply bg-surface border-0;
	}
	:global(.svelte-flow__controls-button:hover) {
		@apply bg-surface-hover;
	}
	:global(.svelte-flow__node.selected .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-500;
	}
</style>

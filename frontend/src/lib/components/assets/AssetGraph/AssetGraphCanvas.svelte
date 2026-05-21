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
	import type { AssetGraphResponse, AssetGraphSelection, NativeTriggerKind } from './types'
	import type { RunnableRunState } from './activeRunnables.svelte'
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
			scriptPath: string,
			outputKind: import('./pipelineTemplates').PipelineOutputKind,
			aiPrompt?: string
		) => void
		// Pipeline-wide + node shown at the top of the graph. Picking any
		// kind from the menu invokes this one callback with the chosen
		// trigger source — the page uses it to seed the draft's annotation.
		// `aiPrompt` is set when the user filled the optional prompt field
		// on the path stage; the page is expected to bootstrap the body via
		// AI instead of using the seeded template.
		onAddPipelineScript?: (
			language: import('$lib/gen').ScriptLang,
			path: string,
			source:
				| { kind: 'schedule'; cron: string }
				| { kind: 'webhook' | 'email' | 'kafka' | 'mqtt' | 'nats' | 'postgres' | 'sqs' | 'gcp' },
			outputKind: import('./pipelineTemplates').PipelineOutputKind,
			aiPrompt?: string
		) => void
		// Folder-scoped prefix shown as a read-only chip in the insert menu
		// path input (e.g. `f/{folder}/`). Shared across top + and per-asset +.
		pathPrefix?: string
		// Seeded editable suffix (e.g. `new_pipeline_script`).
		defaultPathSuffix?: string
		// Default cron expression seeded into the top + template.
		defaultScheduleCron?: string
		// Page-supplied dispatch for the per-asset run button. Receives the
		// producer (kind/path/unsaved) and returns the new job id. The page
		// implements the unsaved branch by calling runScriptPreview with the
		// locally-cached draft content so users can run drafts before
		// deployment. Without this callback, the per-asset run button is
		// hidden.
		onRunProducer?: (producer: {
			kind: 'script' | 'flow'
			path: string
			unsaved?: boolean
			// Whether to let the asset-trigger cascade fan out to downstream
			// subscribers after this run succeeds. Undefined = use the caller's
			// default (no skip arg injected); true = explicit cascade; false =
			// inject `_wmill_skip_asset_dispatch: true` to suppress.
			cascade?: boolean
		}) => Promise<string | undefined>
		// Page-supplied dispatch for the per-runnable-node action menu.
		// Drafts are discarded immediately by the page; persisted scripts
		// are routed through the details pane's archive/delete confirmation
		// flow. Without this callback, the node menu button is hidden.
		onRunnableMenuRemove?: (info: {
			runnable_kind: 'script' | 'flow'
			path: string
			unsaved?: boolean
		}) => void
		// Runnable currently executing — its incoming asset/trigger edges and
		// outgoing write edges get an animated stroke to convey "this is
		// running right now". Zero-latency hint for the script the user just
		// launched (set/cleared by the page from the in-pane Test state).
		activeRunnable?: { kind: 'script' | 'flow'; path: string } | undefined
		// `${kind}:${path}` ids of every pipeline runnable with an in-flight
		// (or just-finished) job, from the folder-scoped queue poll. This is
		// what lights up the *downstream cascade* (jobs the user didn't launch
		// directly). Merged with `activeRunnable` so the launched script lights
		// instantly while cascade hops light as the poll observes them.
		activeRunnableIds?: ReadonlySet<string>
		// `${kind}:${path}` → last-run status + session run count, rendered as
		// a small badge on each runnable node. Same poll source as
		// `activeRunnableIds`; persists the last status while idle.
		runStates?: ReadonlyMap<string, RunnableRunState>
		// Click handler for a "missing trigger" placeholder. The page wires
		// this to its native trigger drawer set so clicking the red node
		// opens the matching editor with `script_path` pre-filled — no
		// navigation, drafts stay intact.
		onCreateMissingTrigger?: (kind: NativeTriggerKind, scriptPath: string) => void
	}
	let {
		graph,
		selection,
		onselect,
		onAddScriptForAsset,
		onAddPipelineScript,
		pathPrefix = '',
		defaultPathSuffix = '',
		defaultScheduleCron = '',
		onRunProducer,
		onRunnableMenuRemove,
		activeRunnable,
		activeRunnableIds,
		runStates,
		onCreateMissingTrigger
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
		// Edge from a missing-trigger placeholder — styled red dashed to
		// signal "this script declared `// on kafka` but no trigger row
		// targets it; create one or remove the annotation".
		missing?: boolean
	}

	// Graph-id of the script the user just launched (zero-latency hint),
	// computed once and reused by the optimistic badge + the active-edge set.
	let activeRunnableNodeId = $derived(
		activeRunnable ? `${activeRunnable.kind}:${activeRunnable.path}` : undefined
	)

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

		const hasAddNode = onAddPipelineScript != null
		if (hasAddNode) {
			nodes.push({
				id: ADD_NODE_ID,
				type: 'add',
				data: {
					onAddPipelineScript: onAddPipelineScript!,
					pathPrefix,
					defaultPathSuffix,
					defaultScheduleCron
				}
			})
		}

		// Producers per asset, derived from write/rw edges. Used by the
		// asset node to surface a "Run" button on hover/select that fires
		// the upstream script(s). Drafts are *included* (with `unsaved:
		// true`) so the button is visible even before the producer is
		// deployed — the click handler shows a "Save first" toast in that
		// case rather than 404'ing. Hiding the button entirely for unsaved
		// producers made the affordance vanish whenever the user only had
		// drafts in scope, which is the common case in a fresh pipeline.
		const producersByAsset = new Map<
			string,
			Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }>
		>()
		for (const e of g.edges ?? []) {
			const access = e.access_type ?? 'r'
			if (access !== 'w' && access !== 'rw') continue
			const key = `${e.asset_kind}:${e.asset_path}`
			const list = producersByAsset.get(key) ?? []
			if (!list.some((p) => p.kind === e.runnable_kind && p.path === e.runnable_path)) {
				list.push({ kind: e.runnable_kind, path: e.runnable_path, unsaved: e.unsaved })
			}
			producersByAsset.set(key, list)
		}

		// Downstream subscriber count per producer script. Counts distinct
		// script subscribers (excluding self and flow subs, mirroring the V1
		// dispatch policy) across all assets the script writes. Drives the
		// "Run + trigger N downstream" menu item on RunnableNode and lets the
		// pipeline page short-circuit cascade UX when there's nothing to fan
		// out to.
		const subscribersByAsset = new Map<string, Set<string>>()
		for (const t of g.triggers ?? []) {
			if (t.trigger_kind !== 'asset' || t.runnable_kind !== 'script') continue
			const key = `${t.asset_kind}:${t.asset_path}`
			const set = subscribersByAsset.get(key) ?? new Set<string>()
			set.add(t.runnable_path)
			subscribersByAsset.set(key, set)
		}
		const downstreamSetsByScript = new Map<string, Set<string>>()
		for (const e of g.edges ?? []) {
			if (e.runnable_kind !== 'script') continue
			const access = e.access_type ?? 'r'
			if (access !== 'w' && access !== 'rw') continue
			const subs = subscribersByAsset.get(`${e.asset_kind}:${e.asset_path}`)
			if (!subs) continue
			const merged = downstreamSetsByScript.get(e.runnable_path) ?? new Set<string>()
			for (const s of subs) if (s !== e.runnable_path) merged.add(s)
			downstreamSetsByScript.set(e.runnable_path, merged)
		}
		const downstreamByScript = new Map<string, number>()
		for (const [path, set] of downstreamSetsByScript) downstreamByScript.set(path, set.size)

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
					defaultPathSuffix,
					producers: producersByAsset.get(`${a.kind}:${a.path}`) ?? [],
					onRunProducer
				}
			})
		}
		for (const r of g.runnables) {
			const rid = `${r.usage_kind}:${r.path}`
			// Optimistic badge: the moment a run is launched from this view
			// `activeRunnable` flips (zero latency), well before the 3s queue
			// poll observes the job. Show "running" immediately, keeping the
			// poll's known run count. Falls back to the polled state (which
			// carries the cascade + the final success/failure) otherwise.
			const polledRunState = runStates?.get(rid)
			let runState = polledRunState
			if (activeRunnableNodeId === rid) {
				runState = { status: 'running' as const, runs: polledRunState?.runs ?? 0 }
			}
			nodes.push({
				id: rid,
				type: 'runnable',
				data: {
					runnable_kind: r.usage_kind,
					path: r.path,
					in_pipeline: r.in_pipeline ?? false,
					partition_kind: r.partition_kind,
					freshness: r.freshness,
					unsaved: r.unsaved ?? false,
					// Same dispatch the asset node uses, only routed when the
					// runnable is a script (the page handler short-circuits
					// flows). The run button mirrors the asset-node affordance:
					// click → run with no extra UI clutter.
					onRunSelf:
						r.usage_kind === 'script' && onRunProducer
							? (opts?: { cascade?: boolean }) =>
									onRunProducer({
										kind: 'script',
										path: r.path,
										unsaved: r.unsaved ?? false,
										cascade: opts?.cascade
									})
							: undefined,
					downstreamCount: downstreamByScript.get(r.path) ?? 0,
					runState,
					onRequestRemove: onRunnableMenuRemove
						? () =>
								onRunnableMenuRemove({
									runnable_kind: r.usage_kind,
									path: r.path,
									unsaved: r.unsaved ?? false
								})
						: undefined
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
		// above the pipeline script. Real (non-missing) nodes are
		// deduplicated per (kind, ref) tuple so a single schedule shared
		// across multiple scripts shows as one node with N outgoing edges.
		// "missing" placeholders are scoped per-(kind, script) — each script
		// gets its own placeholder so the prompt "create / delete" tells
		// the user which script the annotation lives on.
		const triggerSourceNodes = new Map<
			string,
			{
				allUnsaved: boolean
				kind: TriggerNodeKind
				ref: string
				missing: boolean
				runnable_path?: string
			}
		>()
		function recordSourceTrigger(
			id: string,
			kind: TriggerNodeKind,
			ref: string,
			unsaved: boolean,
			missing: boolean,
			runnable_path?: string
		) {
			const prev = triggerSourceNodes.get(id)
			if (!prev) {
				triggerSourceNodes.set(id, { allUnsaved: unsaved, kind, ref, missing, runnable_path })
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
			const isMissing = t.trigger_kind !== 'schedule' && (t as any).missing === true
			// Schedule: cron is the ref. Native (attached): trigger row path.
			// Native (missing): synthesize a per-script ref so each placeholder
			// is its own node ("missing kafka on f/foo/bar").
			const ref = isMissing
				? `missing:${t.runnable_path}`
				: t.trigger_kind === 'schedule'
					? (t as any).cron
					: ((t as any).path ?? '')
			const sourceId = `trigger:${t.trigger_kind}:${ref}`
			recordSourceTrigger(
				sourceId,
				t.trigger_kind,
				ref,
				!!t.unsaved,
				isMissing,
				isMissing ? t.runnable_path : undefined
			)
			edges.push({
				id: `trig-${t.trigger_kind}:${sourceId}->${runnableId}`,
				source: sourceId,
				target: runnableId,
				kind: t.trigger_kind === 'schedule' ? 'trigger-schedule' : 'trigger-native',
				unsaved: t.unsaved,
				missing: isMissing
			})
		}
		for (const [id, info] of triggerSourceNodes) {
			nodes.push({
				id,
				type: 'trigger',
				data: {
					kind: info.kind,
					ref: info.ref,
					unsaved: info.allUnsaved,
					missing: info.missing,
					runnable_path: info.runnable_path,
					onCreateMissingTrigger
				}
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

	// Graph-ids of every runnable that's executing right now: the polled
	// in-flight/just-finished set (cascade) plus the zero-latency hint for
	// the script the user just launched. Edges into/out of any of these get
	// the animated stroke — animation stays reserved as an "is happening
	// now" signal and the static graph stays quiet when nothing runs.
	let activeRunnableIdSet = $derived(
		new Set<string>([
			...(activeRunnableIds ?? []),
			...(activeRunnableNodeId ? [activeRunnableNodeId] : [])
		])
	)
	let flowEdges = $derived.by(() =>
		model.edges
			// Anchor edges are layout-only.
			.filter((e) => e.kind !== 'add-anchor')
			.map<Edge>((e) => {
				const touchesActiveRunnable =
					activeRunnableIdSet.size > 0 &&
					(activeRunnableIdSet.has(e.source) || activeRunnableIdSet.has(e.target))
				let style: string
				let animated = false
				let markerColor: string | undefined = undefined
				let strokeDasharray: string | undefined = undefined
				let label: string | undefined = undefined
				let labelStyle: string | undefined = undefined
				switch (e.kind) {
					case 'lineage-write':
						style = 'stroke: rgb(59 130 246); stroke-width: 1.5px;'
						animated = touchesActiveRunnable
						break
					case 'lineage-read':
						style = 'stroke: rgb(107 114 128); stroke-width: 1.25px;'
						animated = touchesActiveRunnable
						break
					case 'trigger-asset':
						style = 'stroke: rgb(16 185 129); stroke-width: 2px;'
						animated = touchesActiveRunnable
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
				//
				// `animated` is intentionally NOT cleared here: a live-parsed
				// output edge still belongs to a script that can be run, and
				// the run feedback (flow animation toward its output assets)
				// is exactly what the user expects while it executes. The
				// dashed+dimmed treatment already conveys "not persisted";
				// it composes fine with the active-run animation.
				if (e.unsaved) {
					strokeDasharray = '3 3'
					style = `${style} opacity: 0.7;`
					if (label) label = `${label} (unsaved)`
				}
				// Missing-trigger edge: overrides the per-kind stroke colour
				// with red so the entire "annotated but no row" branch reads
				// as broken at a glance. Composes with `unsaved` if both
				// (red dashed dimmed — fresh draft annotation that also has
				// no matching row, which is the common case).
				if (e.missing) {
					style = 'stroke: rgb(239 68 68); stroke-width: 2px;'
					strokeDasharray = '3 3'
					markerColor = 'rgb(239 68 68)'
					label = 'missing trigger'
					labelStyle = 'fill: rgb(239 68 68); font-size: 10px; font-weight: 600;'
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
		--background-color={false}
	>
		<div class="absolute inset-0 !bg-surface-secondary h-full"></div>
		<Controls position="top-right" orientation="horizontal" showLock={false} class="!mr-10" />
		<MiniMap
			pannable
			zoomable
			class="!bg-surface !mb-10"
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

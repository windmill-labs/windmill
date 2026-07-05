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
	import DataTestNode from './DataTestNode.svelte'
	import AssetGraphEdge from './AssetGraphEdge.svelte'
	import PanToNode from './PanToNode.svelte'
	import InitialFitView from './InitialFitView.svelte'
	import { layoutAssetGraph } from './assetGraphLayout'
	import { buildDownstreamMap } from './graphTraversal'
	import { buildLineageDownstreamMap } from './boundedCascade'
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
			source: {
				kind: NativeTriggerKind
				path: string | undefined
			},
			outputKind: import('./pipelineTemplates').PipelineOutputKind,
			aiPrompt?: string
		) => void
		// Folder-scoped prefix shown as a read-only chip in the insert menu
		// path input (e.g. `f/{folder}/`). Shared across top + and per-asset +.
		pathPrefix?: string
		// Seeded editable suffix (e.g. `new_pipeline_script`).
		defaultPathSuffix?: string
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
		// Click handler for an attached (non-missing) native trigger node —
		// opens the matching editor in edit mode for the given trigger path.
		// `scriptPath` is the related script — the drawer locks its
		// script-picker to it so the trigger can't be reassigned off the
		// pipeline from this entry point.
		onEditTrigger?: (kind: NativeTriggerKind, triggerPath: string, scriptPath: string) => void
		// Click handler for the kebab → Delete entry on an attached trigger.
		// The page is expected to confirm + call the matching delete API
		// and refetch the graph.
		onDeleteTrigger?: (kind: NativeTriggerKind, triggerPath: string) => void
		// Click handler for a webhook node — opens the webhook drawer (URLs +
		// webhook-specific token creation) for the given script. Webhooks have
		// no trigger row, so they don't use the create/edit/delete flows.
		onOpenWebhook?: (scriptPath: string) => void
		// Click handler for a data_upload node — opens the target script's run
		// form (with the auto-generated S3 picker) for the given script.
		// data_upload has no trigger row; it's a UI-first entry point.
		onOpenDataUpload?: (scriptPath: string) => void
		// Script paths the cursor is over in the Activity panel — a thin neutral
		// ring each. One entry for a single run row; the whole cascade's runs
		// when hovering a group header. Distinct from `selectedRunPaths`.
		hoveredPaths?: string[]
		// Script paths of the expanded (pinned) run(s) — a soft blue ring that
		// persists until collapsed.
		selectedRunPaths?: string[]
		// Graph-id (`${kind}:${path}`) of a node to smoothly pan into the
		// center of the viewport — set by the page when a new draft node is
		// created so the user's eye follows it. Opt-in: only the pipeline
		// editor passes it, so the asset-graph page is unaffected. The page
		// clears it once the pan has had time to settle.
		panToNodeId?: string | undefined
		// Script paths eligible to *start* a bounded-cascade run (schedule
		// roots + manual roots — see boundedCascade.validStarts). When a path is
		// in this set and `onStartBoundedRun` is wired, its node's cascade menu
		// gains a "Run downstream up to…" entry.
		validStartPaths?: ReadonlySet<string>
		onStartBoundedRun?: (startPath: string) => void
		// Active bounded-run pick mode. Ids are in *canvas* space (`script:path`
		// for runnables, `asset:${kind}:${path}` for assets). When set the canvas
		// dims nodes outside `eligible ∪ {start}`, rings the `bounded` set, marks
		// `ends`, and routes clicks on eligible nodes to `onPickEnd` instead of
		// selecting.
		boundPick?: {
			start: string
			eligible: ReadonlySet<string>
			ends: ReadonlySet<string>
			bounded: ReadonlySet<string>
		}
		onPickEnd?: (canvasNodeId: string) => void
		/** Hide the minimap when the canvas is too narrow for it to be worth the
		 * space (e.g. stacked layout in a side panel). Defaults to shown. */
		showMinimap?: boolean
		/** Identity of the displayed graph (e.g. the pipeline folder). The
		 * initial viewport fit re-arms when it changes, so switching folders
		 * in-place gets a fresh fit. */
		viewportFitKey?: string
	}
	let {
		graph,
		selection,
		onselect,
		onAddScriptForAsset,
		onAddPipelineScript,
		pathPrefix = '',
		defaultPathSuffix = '',
		onRunProducer,
		onRunnableMenuRemove,
		activeRunnable,
		activeRunnableIds,
		runStates,
		onCreateMissingTrigger,
		onEditTrigger,
		onDeleteTrigger,
		onOpenWebhook,
		onOpenDataUpload,
		hoveredPaths,
		selectedRunPaths,
		panToNodeId,
		validStartPaths,
		onStartBoundedRun,
		boundPick,
		onPickEnd,
		showMinimap = true,
		viewportFitKey = ''
	}: Props = $props()

	// `${kind}:${path}` ids for the hovered / pinned runs (both script and flow
	// variants, since the run row's kind isn't known here).
	const runIds = (paths: string[] | undefined): Set<string> =>
		new Set((paths ?? []).flatMap((p) => [`script:${p}`, `flow:${p}`]))
	let hoveredRunIdSet = $derived(runIds(hoveredPaths))
	let selectedRunIdSet = $derived(runIds(selectedRunPaths))

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
			| 'trigger-native'
			| 'add-anchor'
			| 'data-test'
			| 'macro'
			| 'test-dependency'
		unsaved?: boolean
		// Edge from a missing-trigger placeholder — styled red dashed to
		// signal "this script declared `// on kafka` but no trigger row
		// targets it; create one or remove the annotation".
		missing?: boolean
		// Producer's `// data_test` checks, on the write-edge to the
		// materialized asset — rendered as a flask badge on the link.
		data_tests?: NonNullable<AssetGraphResponse['runnables'][number]['data_tests']>
		// Producer's `// column` declared lineage, on the same write-edge —
		// rendered as a columns badge on the link.
		column_lineage?: NonNullable<AssetGraphResponse['runnables'][number]['column_lineage']>
		// Macro-library edge payload: the macros the consumer calls (or the
		// whole lib when pulled in via `// use`) — rendered as a ƒ badge.
		macro_names?: string[]
		via_use?: boolean
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
			type: 'asset' | 'runnable' | 'trigger' | 'add' | 'data-test'
			data: any
		}> = []
		const edges: BuiltEdge[] = []
		// Custom (`// data_test <script_path>`) tests are deployed scripts, so we
		// draw each as its own node hanging off the asset it validates (deduped
		// by node id across producers).
		const addedTestNodes = new Set<string>()

		const hasAddNode = onAddPipelineScript != null
		if (hasAddNode) {
			nodes.push({
				id: ADD_NODE_ID,
				type: 'add',
				data: {
					onAddPipelineScript: onAddPipelineScript!,
					pathPrefix,
					defaultPathSuffix
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
		// out to. Shared with the dev-run cascade orchestrator (graphTraversal).
		const downstreamByScript = new Map<string, number>()
		// How many of those subscribers are unsaved drafts — the production
		// dispatcher can't reach them, so the cascade menu item flags that the
		// run is a dev orchestration / that deploy is needed for auto-trigger.
		const downstreamUnsavedByScript = new Map<string, number>()
		const unsavedRunnables = new Set(
			(g.runnables ?? []).filter((r) => r.unsaved).map((r) => r.path)
		)
		for (const [path, set] of buildDownstreamMap(g)) {
			downstreamByScript.set(path, set.size)
			downstreamUnsavedByScript.set(path, [...set].filter((s) => unsavedRunnables.has(s)).length)
		}
		// Read-aware downstream presence for the bounded-run gate. The bounded
		// engine treats pure-read `asset → script` edges as downstream, but the
		// subscriber-only `downstreamByScript` above does not — so a start whose
		// only downstream is a pure reader would otherwise be denied the menu
		// item even though its bounded set is non-empty. Mirror of the engine's
		// own adjacency (boundedCascade.buildLineageDownstreamMap).
		const hasLineageDownstream = new Set<string>(buildLineageDownstreamMap(g).keys())

		for (const a of g.assets) {
			const assetId = `asset:${a.kind}:${a.path}`
			nodes.push({
				id: assetId,
				type: 'asset',
				data: {
					asset_kind: a.kind,
					path: a.path,
					fork_materialization: a.fork_materialization,
					derived_from: a.derived_from,
					onAddScript: onAddScriptForAsset,
					pathPrefix,
					defaultPathSuffix,
					producers: producersByAsset.get(`${a.kind}:${a.path}`) ?? [],
					onRunProducer
				}
			})
		}
		// Set of `script_path` values for runnables that are still drafts (no
		// DB row yet). Trigger nodes use this to swap "Click to create" for
		// "Click to create (after draft save)" so the user knows the create
		// button is blocked until the script is deployed.
		const unsavedRunnablePaths = new Set<string>()
		for (const r of g.runnables) {
			if (r.unsaved) unsavedRunnablePaths.add(r.path)
		}
		// Producer → its `// data_test` checks, keyed by runnable id, so the
		// write-edge to the materialized asset can carry the test badge: the
		// edge *is* the transformation, and the tests assert on what it produces.
		const producerTests = new Map<
			string,
			NonNullable<AssetGraphResponse['runnables'][number]['data_tests']>
		>()
		for (const r of g.runnables) {
			if (r.data_tests && r.data_tests.length > 0) {
				producerTests.set(`${r.usage_kind}:${r.path}`, r.data_tests)
			}
		}
		// Producer → its `// column` declared lineage, keyed by runnable id, so
		// the write-edge to the materialized asset can carry the columns badge —
		// the edge *is* the transformation, and the lineage describes its output.
		const producerColumnLineage = new Map<
			string,
			NonNullable<AssetGraphResponse['runnables'][number]['column_lineage']>
		>()
		// The `// materialize` target the lineage describes, so the badge lands
		// only on that write-edge (a multi-output script writes several ducklake
		// tables) — mirrors the anchor logic in `buildColumnGraph`.
		const producerMaterializeTarget = new Map<
			string,
			NonNullable<AssetGraphResponse['runnables'][number]['materialize_target']>
		>()
		for (const r of g.runnables) {
			if (r.column_lineage && r.column_lineage.length > 0) {
				producerColumnLineage.set(`${r.usage_kind}:${r.path}`, r.column_lineage)
			}
			if (r.materialize_target) {
				producerMaterializeTarget.set(`${r.usage_kind}:${r.path}`, r.materialize_target)
			}
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
					last_success_at: r.last_success_at,
					tag: r.tag,
					retry: r.retry,
					macros: r.macros,
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
					downstreamUnsavedCount: downstreamUnsavedByScript.get(r.path) ?? 0,
					runState,
					// Bounded-cascade entrypoint: only valid starts (schedule /
					// manual roots) with downstream get the "Run downstream up
					// to…" menu item.
					onStartBoundedRun:
						r.usage_kind === 'script' &&
						onStartBoundedRun &&
						validStartPaths?.has(r.path) &&
						hasLineageDownstream.has(r.path)
							? () => onStartBoundedRun(r.path)
							: undefined,
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
				// Data tests assert on the `// materialize` target, which is always
				// a ducklake asset (v1 enforces this), so only the ducklake
				// write-edge carries the badge / custom-test nodes — a producer's
				// other (e.g. S3/datatable) outputs must not show them.
				const edgeTests = e.asset_kind === 'ducklake' ? producerTests.get(runnableId) : undefined
				// Column lineage describes one materialized output, so the badge
				// lands only on that asset's write-edge: the declared `// materialize`
				// target when known (a multi-output script writes several ducklake
				// tables), else the ducklake write-edge as the unambiguous fallback.
				const matTarget = producerMaterializeTarget.get(runnableId)
				const isOutputEdge = matTarget
					? e.asset_kind === matTarget.kind && e.asset_path === matTarget.path
					: e.asset_kind === 'ducklake'
				const edgeColumnLineage = isOutputEdge ? producerColumnLineage.get(runnableId) : undefined
				edges.push({
					id: `prod:${runnableId}->${assetId}`,
					source: runnableId,
					target: assetId,
					kind: 'lineage-write',
					unsaved: e.unsaved,
					data_tests: edgeTests,
					column_lineage: edgeColumnLineage
				})
				// Each custom (`// data_test <script_path>`) test → its own node
				// below the asset it validates, with a dashed "tests" edge.
				for (const t of edgeTests ?? []) {
					if (t.type !== 'custom') continue
					const testNodeId = `datatest:${assetId}:${t.path}`
					if (!addedTestNodes.has(testNodeId)) {
						addedTestNodes.add(testNodeId)
						nodes.push({ id: testNodeId, type: 'data-test', data: { path: t.path } })
					}
					edges.push({
						id: `test:${assetId}->${testNodeId}`,
						source: assetId,
						target: testNodeId,
						kind: 'data-test',
						unsaved: e.unsaved
					})
				}
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

		// Macro-library → consumer edges (runnable→runnable, unlike the
		// asset-mediated lineage above). Endpoints must exist as nodes — an
		// undeployed `// use` target without a draft has none, so its edge is
		// skipped (the annotation still shows in the script body itself).
		const runnableNodeIds = new Set(g.runnables.map((r) => `${r.usage_kind}:${r.path}`))
		for (const me of g.macro_edges ?? []) {
			const libId = `script:${me.lib_path}`
			const consumerId = `script:${me.consumer_path}`
			if (!runnableNodeIds.has(libId) || !runnableNodeIds.has(consumerId)) continue
			edges.push({
				id: `macro:${libId}->${consumerId}`,
				source: libId,
				target: consumerId,
				kind: 'macro',
				unsaved: me.unsaved,
				macro_names: me.macro_names,
				via_use: me.via_use
			})
		}

		// Data-test ordering edges (producer → tested script): the referenced
		// asset must be materialized before the test runs, so the cascade orders
		// the producer first. Rendered as a dashed "must run after" link, distinct
		// from data flow — the tested script doesn't consume the asset's rows.
		for (const te of g.test_edges ?? []) {
			const producerId = `${te.producer_kind}:${te.producer_path}`
			const testedId = `${te.runnable_kind}:${te.runnable_path}`
			if (!runnableNodeIds.has(producerId) || !runnableNodeIds.has(testedId)) continue
			edges.push({
				id: `testdep:${producerId}->${testedId}`,
				source: producerId,
				target: testedId,
				kind: 'test-dependency'
			})
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
				// First target script (drives the per-script create/edit flows).
				runnable_path?: string
				// Every target script: a single (kind, ref) — e.g. one schedule —
				// can be shared across scripts and dedupes to one node with N
				// edges. The bounded-run action must consider all of them, not
				// just the first.
				runnable_paths: string[]
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
				triggerSourceNodes.set(id, {
					allUnsaved: unsaved,
					kind,
					ref,
					missing,
					runnable_path,
					runnable_paths: runnable_path ? [runnable_path] : []
				})
			} else {
				prev.allUnsaved = prev.allUnsaved && unsaved
				if (runnable_path && !prev.runnable_paths.includes(runnable_path)) {
					prev.runnable_paths.push(runnable_path)
				}
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
			// Rowless kinds (webhook, data_upload) never have a trigger row by
			// design — webhook is an implicit endpoint, data_upload is a
			// UI-first entry point. They're per-script source nodes, not
			// "missing" placeholders, so suppress the missing/red treatment on
			// both node and edge while keeping a per-script identity.
			const isRowless = t.trigger_kind === 'webhook' || t.trigger_kind === 'data_upload'
			const isMissing = (t as any).missing === true && !isRowless
			// Attached native: trigger row path. Rowless: per-script ref so each
			// script gets its own node. Missing: synthesize a per-script ref so
			// each placeholder is its own node ("missing kafka on f/foo/bar").
			// Schedule joins the native family — its ref is the schedule row's
			// path, same shape.
			const ref = isRowless
				? `rowless:${t.runnable_path}`
				: isMissing
					? `missing:${t.runnable_path}`
					: ((t as any).path ?? '')
			const sourceId = `trigger:${t.trigger_kind}:${ref}`
			recordSourceTrigger(
				sourceId,
				t.trigger_kind,
				ref,
				!!t.unsaved,
				isMissing,
				// Always thread the target script so the trigger node can
				// reach back to it — drives both the missing-trigger "create"
				// flow and the attached-trigger "edit" flow's script-path
				// lock. Previously only set for missing, which silently
				// disabled `canEdit` (and the resulting click affordance)
				// on every attached native trigger.
				t.runnable_path
			)
			edges.push({
				id: `trig-${t.trigger_kind}:${sourceId}->${runnableId}`,
				source: sourceId,
				target: runnableId,
				kind: 'trigger-native',
				unsaved: t.unsaved,
				missing: isMissing
			})
		}
		for (const [id, info] of triggerSourceNodes) {
			// Bounded-run targets among this trigger's scripts: valid starts that
			// also have read-aware downstream. A shared schedule may have several
			// targets; only offer the action when exactly one qualifies, so the
			// run starts from an unambiguous script (rather than the arbitrary
			// first-seen one). Multi-eligible nodes suppress it rather than guess.
			const eligibleStarts = onStartBoundedRun
				? info.runnable_paths.filter((p) => validStartPaths?.has(p) && hasLineageDownstream.has(p))
				: []
			nodes.push({
				id,
				type: 'trigger',
				data: {
					kind: info.kind,
					ref: info.ref,
					unsaved: info.allUnsaved,
					missing: info.missing,
					runnable_path: info.runnable_path,
					runnable_unsaved: info.runnable_path
						? unsavedRunnablePaths.has(info.runnable_path)
						: false,
					onCreateMissingTrigger,
					onEditTrigger,
					onDeleteTrigger,
					onOpenWebhook,
					onOpenDataUpload,
					// View-mode bounded-run entry: offer it on the trigger node
					// when exactly one target script is a valid start with
					// downstream (see eligibleStarts above).
					onStartBoundedRun:
						onStartBoundedRun && eligibleStarts.length === 1
							? () => onStartBoundedRun(eligibleStarts[0])
							: undefined
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

	// Compute layout positions strictly off the model — selection changing
	// must NOT trigger a re-layout. Sugiyama's decross output depends on
	// the order in which nodes/edges arrive, so two builds with identical
	// topology but different insertion order can produce different layer
	// orderings (selecting a node briefly re-emits `liveAnnotations`-
	// driven trigger overlays in a different order, etc.). Sorting both
	// arrays by their stable identity (node id, edge source→target)
	// gives sugiyama a topology+path-deterministic input — same nodes +
	// edges + paths → same layout. Renames *do* change the layout for
	// the renamed entry, since its id moves in the sort, but that's
	// expected: a rename is a path change, which is part of the input.
	// A script with rw access to an asset yields both a write (script → asset)
	// and a read/trigger (asset → script) edge — a 2-cycle. The layout resolves
	// it producer-above-asset by omitting the backward direction from its
	// input; the rendered edges are untouched (both arrows still drawn).
	let writeEdgePairs = $derived(
		new Set(
			model.edges.filter((e) => e.kind === 'lineage-write').map((e) => `${e.source}\n${e.target}`)
		)
	)
	let layoutInput = $derived({
		nodes: model.nodes
			.map((n) => ({ id: n.id, data: n.data }))
			.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)),
		edges: model.edges
			.filter(
				(e) =>
					!(
						(e.kind === 'lineage-read' || e.kind === 'trigger-asset') &&
						writeEdgePairs.has(`${e.target}\n${e.source}`)
					)
			)
			.map((e) => ({ source: e.source, target: e.target }))
			.sort((a, b) =>
				a.source === b.source
					? a.target < b.target
						? -1
						: a.target > b.target
							? 1
							: 0
					: a.source < b.source
						? -1
						: 1
			)
	})
	let layoutPositions = $derived(layoutAssetGraph(layoutInput, ADD_NODE_ID))

	let positionedNodes = $derived.by(() => {
		// Compute bbox width from layout; shift every x so the graph is
		// horizontally centered inside the pane. y is untouched so layer 0
		// sits at the top of the viewport (matches flow editor's Trigger
		// placement — no fitView reshuffling).
		let minX = Infinity
		let maxX = -Infinity
		for (const p of layoutPositions.values()) {
			if (p.x < minX) minX = p.x
			if (p.x > maxX) maxX = p.x
		}
		const bboxWidth = isFinite(minX) ? maxX - minX : 0
		const xCenter = paneWidth / 2 - bboxWidth / 2
		return model.nodes.map<Node>((n) => {
			const p = layoutPositions.get(n.id) ?? { x: 0, y: 0 }
			// Compensate for the + node being narrower than its layout slot
			// so it visually centers over the node(s) below.
			const xShift = n.id === ADD_NODE_ID ? (NODE.width - ADD_NODE_WIDTH) / 2 : 0
			// Activity-panel emphasis (purely visual rings, kept off `selected`
			// which swaps the details pane). Hover wins over pin so the cursor
			// always tracks.
			const runClass = hoveredRunIdSet.has(n.id)
				? 'wm-run-hover'
				: selectedRunIdSet.has(n.id)
					? 'wm-run-selected'
					: undefined
			// Asset border matching the connecting edge's hue (only when no run
			// class applies — a node is either a runnable or an asset).
			const assetEmph = assetEmphasis.get(n.id)
			const assetClass =
				assetEmph === 'output'
					? 'wm-asset-output'
					: assetEmph === 'input'
						? 'wm-asset-input'
						: undefined
			// Bounded-run pick overlay takes precedence over the activity rings
			// (it's a transient, modal selection). start ring > end mark > in-set
			// ring > eligible (clickable, no style) > dimmed (out of reach).
			let boundClass: string | undefined
			if (boundPick && n.type !== 'add' && n.type !== 'trigger') {
				if (n.id === boundPick.start) boundClass = 'wm-bound-start'
				else if (boundPick.ends.has(n.id)) boundClass = 'wm-bound-end'
				else if (boundPick.bounded.has(n.id)) boundClass = 'wm-bound-in'
				else if (!boundPick.eligible.has(n.id)) boundClass = 'wm-bound-dim'
			}
			return {
				id: n.id,
				type: n.type,
				position: { x: p.x + xCenter + xShift, y: p.y + 40 },
				data: n.data,
				class: boundClass ?? runClass ?? assetClass,
				selected: n.id === selectedId,
				// All nodes non-draggable: the layout is sugiyama-computed,
				// dragging would fight the reactive re-layout. Selection is
				// still allowed on asset/runnable (not on the + or schedules)
				// for the details-pane click-through. Trigger nodes opt out
				// of selection but TriggerNode itself sets `pointer-events: auto`
				// on its inner box so the edit/create button still receives
				// clicks despite svelte-flow's wrapper-level pointer-events: none.
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
	// Node ids emphasized from the Activity panel (hovered or pinned runs).
	// Their incident edges get the same flow animation as a live run, so
	// hovering a run (or a whole group) traces its data path through the graph.
	let emphasizedRunIdSet = $derived(new Set<string>([...hoveredRunIdSet, ...selectedRunIdSet]))
	// Asset nodes adjacent to an emphasized run, tagged by edge family so their
	// border matches the edge hue: `output` (a write, blue) / `input` (a read or
	// asset-trigger, gray). Output wins if an asset is both.
	let assetEmphasis = $derived.by<Map<string, 'input' | 'output'>>(() => {
		const m = new Map<string, 'input' | 'output'>()
		if (emphasizedRunIdSet.size === 0) return m
		for (const e of model.edges) {
			if (e.kind === 'lineage-write' && emphasizedRunIdSet.has(e.source)) {
				m.set(e.target, 'output')
			} else if (
				(e.kind === 'lineage-read' || e.kind === 'trigger-asset') &&
				emphasizedRunIdSet.has(e.target) &&
				m.get(e.source) !== 'output'
			) {
				m.set(e.source, 'input')
			}
		}
		return m
	})
	// Node centers (handle x ≈ left + width/2; y = top) for obstacle-aware edge
	// routing. Computed off the laid-out positions, so it only re-runs when the
	// graph relayouts — never per frame.
	let nodeCenters = $derived(
		positionedNodes
			.filter((n) => n.id !== ADD_NODE_ID)
			.map((n) => ({ id: n.id, cx: n.position.x + NODE.width / 2, cy: n.position.y }))
	)
	// Detour lane for an edge whose straight run would pass over an unrelated
	// node (the failure the same-column gutter can't see). For each node
	// strictly between the endpoints' rows, sample the straight line at that
	// row; if the node sits under it, route around the obstacle on the side the
	// edge is already heading. Returns the outermost lane x clearing every
	// crossed node, or undefined when the corridor is clear. O(nodes) per edge.
	const HALF_W = NODE.width / 2
	const ROUTE_PAD = NODE.gap.horizontal / 2
	function detourForEdge(sourceId: string, targetId: string): number | undefined {
		const s = nodeCenters.find((n) => n.id === sourceId)
		const t = nodeCenters.find((n) => n.id === targetId)
		if (!s || !t || s.cy === t.cy) return undefined
		const dyTot = t.cy - s.cy
		let lane: number | undefined
		for (const n of nodeCenters) {
			if (n.id === sourceId || n.id === targetId) continue
			// strictly between the two rows
			if ((n.cy - s.cy) / dyTot <= 0.01 || (n.cy - s.cy) / dyTot >= 0.99) continue
			const edgeX = s.cx + (t.cx - s.cx) * ((n.cy - s.cy) / dyTot)
			if (Math.abs(edgeX - n.cx) >= HALF_W + 8) continue
			// Crossed: a lane just outside this node, toward the target side.
			const side = t.cx >= n.cx ? 1 : -1
			const candidate = n.cx + side * (HALF_W + ROUTE_PAD)
			// Keep the outermost lane so one detour clears every obstacle.
			if (lane == undefined || Math.abs(candidate - s.cx) > Math.abs(lane - s.cx)) lane = candidate
		}
		return lane
	}

	let flowEdges = $derived.by(() =>
		model.edges
			// Anchor edges are layout-only.
			.filter((e) => e.kind !== 'add-anchor')
			.map<Edge>((e) => {
				const touchesActiveRunnable =
					activeRunnableIdSet.size > 0 &&
					(activeRunnableIdSet.has(e.source) || activeRunnableIdSet.has(e.target))
				// Edge incident to an Activity-panel hovered/pinned run — animate
				// it too so the row↔graph link is unmistakable.
				const touchesEmphasis =
					emphasizedRunIdSet.size > 0 &&
					(emphasizedRunIdSet.has(e.source) || emphasizedRunIdSet.has(e.target))
				const flowAnimated = touchesActiveRunnable || touchesEmphasis
				let style: string
				let animated = false
				let markerColor: string | undefined = undefined
				let strokeDasharray: string | undefined = undefined
				let label: string | undefined = undefined
				let labelStyle: string | undefined = undefined
				// Two edge families, two hues: edges *out of scripts* (writes
				// — data being produced) carry the luminance-blue accent and
				// pair with the blue asset icons; edges *out of data* (reads
				// + asset triggers into scripts) stay gray, with dashes /
				// labels for the trigger variants. Red is reserved for the
				// missing-trigger branch below; the run animation itself is
				// the "happening now" signal.
				switch (e.kind) {
					case 'lineage-write':
						style = 'stroke: rgb(59 130 246); stroke-width: 2px;'
						animated = flowAnimated
						markerColor = 'rgb(59 130 246)'
						break
					case 'lineage-read':
						style = 'stroke: rgb(156 163 175); stroke-width: 1.25px;'
						animated = flowAnimated
						break
					case 'data-test':
						// Asset → its custom-test script: dashed, muted, no run
						// animation (the test isn't a producing step).
						style = 'stroke: rgb(156 163 175); stroke-width: 1.25px;'
						strokeDasharray = '4 3'
						markerColor = 'rgb(156 163 175)'
						break
					case 'trigger-asset':
						style = 'stroke: rgb(107 114 128); stroke-width: 2px;'
						animated = flowAnimated
						markerColor = 'rgb(107 114 128)'
						label = 'triggers'
						labelStyle = 'fill: rgb(107 114 128); font-size: 10px; font-weight: 600;'
						break
					case 'trigger-native':
						style = 'stroke: rgb(107 114 128); stroke-width: 2px;'
						strokeDasharray = '6 3'
						markerColor = 'rgb(107 114 128)'
						label = 'triggers'
						labelStyle = 'fill: rgb(107 114 128); font-size: 10px; font-weight: 600;'
						break
					case 'macro':
						// Library → consumer: violet dashed, visually apart from both
						// lineage (blue/gray solid) and trigger (gray dashed) families —
						// it's a code dependency, not data flow or execution.
						style = 'stroke: rgb(139 92 246); stroke-width: 1.25px;'
						strokeDasharray = '5 3'
						markerColor = 'rgb(139 92 246)'
						label = e.via_use ? 'uses lib' : 'macros'
						labelStyle = 'fill: rgb(139 92 246); font-size: 10px; font-weight: 600;'
						break
					case 'test-dependency':
						// Producer → tested script: amber dashed ordering link. Not
						// data flow (blue/gray) nor execution trigger (gray "triggers")
						// — it only says "the test needs this asset to exist first".
						style = 'stroke: rgb(217 119 6); stroke-width: 1.25px;'
						strokeDasharray = '5 3'
						markerColor = 'rgb(217 119 6)'
						label = 'test needs'
						labelStyle = 'fill: rgb(217 119 6); font-size: 10px; font-weight: 600;'
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
					data: {
						detourX: detourForEdge(e.source, e.target),
						// Data-test badge on the producer→asset write-edge. The
						// producer's last-run status (the script fails if any test
						// fails) tints it green/red; neutral until it has run.
						data_tests: e.data_tests,
						testsRunStatus: e.data_tests?.length ? runStates?.get(e.source)?.status : undefined,
						// Column-lineage badge on the same write-edge (the link is the
						// transformation whose output columns the lineage describes).
						column_lineage: e.column_lineage,
						// Macro-edge badge: which of the library's macros the consumer
						// calls (all of them when pulled in via `// use`).
						macro_names: e.macro_names,
						via_use: e.via_use
					},
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
		add: AddNode as any,
		'data-test': DataTestNode as any
	}

	const edgeTypes = {
		asset: AssetGraphEdge as any
	}

	function handleNodeClick({ node }: { node: Node }) {
		// Bounded-run pick mode intercepts clicks: an eligible (downstream)
		// node toggles as an end bound; the start, dimmed nodes, and
		// triggers/+ are inert. Selection (details pane) is suppressed so the
		// modal pick stays focused.
		if (boundPick && onPickEnd) {
			if (node.type !== 'asset' && node.type !== 'runnable') return
			if (node.id === boundPick.start) return
			if (!boundPick.eligible.has(node.id)) return
			onPickEnd(node.id)
			return
		}
		if (!onselect) return
		const data = node.data as any
		if (node.type === 'asset') {
			onselect({ kind: 'asset', asset_kind: data.asset_kind, path: data.path })
		} else if (node.type === 'runnable') {
			onselect({ kind: 'runnable', runnable_kind: data.runnable_kind, path: data.path })
		} else if (node.type === 'data-test') {
			// A custom test is a deployed script — open it like any runnable.
			onselect({ kind: 'runnable', runnable_kind: 'script', path: data.path })
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
		<InitialFitView {nodes} fitKey={viewportFitKey} />
		<PanToNode targetId={panToNodeId} {nodes} />
		<Controls position="top-right" orientation="horizontal" showLock={false} class="!mr-10" />
		{#if showMinimap}
			<!-- Node hues mirror the canvas: blue asset cards, amber triggers,
			     bordered neutral script cards. Visible strokes + rounded corners +
			     a bordered container + the outlined viewport mask are what make
			     this read as a minimap instead of a loading skeleton. -->
			<MiniMap
				pannable
				zoomable
				class="!bg-surface !mb-10 rounded-md border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden"
				nodeBorderRadius={12}
				nodeStrokeWidth={6}
				nodeColor={(n) =>
					n.type === 'asset'
						? 'rgb(59 130 246 / 0.3)'
						: n.type === 'trigger'
							? 'rgb(245 158 11 / 0.3)'
							: 'rgb(148 163 184 / 0.15)'}
				nodeStrokeColor={(n) =>
					n.type === 'asset'
						? 'rgb(59 130 246 / 0.8)'
						: n.type === 'trigger'
							? 'rgb(245 158 11 / 0.8)'
							: 'rgb(100 116 139 / 0.7)'}
				maskColor="rgb(100 116 139 / 0.12)"
				maskStrokeColor="rgb(59 130 246 / 0.5)"
				maskStrokeWidth={4}
			/>
		{/if}
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
	/* Activity-panel emphasis — soft, monochromatic, less prominent than the
	   blue details selection above. Hover is a thin neutral ring (transient);
	   pinning an expanded run is a soft-blue ring. */
	:global(.svelte-flow__node.wm-run-hover .drop-shadow-sm) {
		@apply outline outline-1 outline-gray-400 dark:outline-gray-500;
	}
	:global(.svelte-flow__node.wm-run-selected .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-400/70;
	}
	/* Assets adjacent to an emphasized run — border hue matches the edge:
	   blue for a write (output), gray for a read / asset-trigger (input). */
	:global(.svelte-flow__node.wm-asset-output .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-400/70;
	}
	:global(.svelte-flow__node.wm-asset-input .drop-shadow-sm) {
		@apply outline outline-2 outline-gray-400;
	}
	/* Bounded-run pick mode. The start is a solid blue ring; chosen end
	   bounds get a thicker amber ring; nodes inside the path-between set ring
	   blue; everything out of reach fades back so the matched subset reads at
	   a glance. */
	:global(.svelte-flow__node.wm-bound-start .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-500;
	}
	:global(.svelte-flow__node.wm-bound-in .drop-shadow-sm) {
		@apply outline outline-2 outline-blue-400/70;
	}
	:global(.svelte-flow__node.wm-bound-end .drop-shadow-sm) {
		@apply outline outline-[3px] outline-amber-500;
	}
	:global(.svelte-flow__node.wm-bound-dim) {
		@apply opacity-30;
	}
</style>

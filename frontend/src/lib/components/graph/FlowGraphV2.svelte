<script lang="ts">
	import {
		AssetService,
		FlowService,
		ResourceService,
		type AssetUsageKind,
		type FlowModule
	} from '../../gen'
	import { NODE, type GraphModuleState } from '.'
	import { getContext, onDestroy, setContext, tick, untrack } from 'svelte'

	import { get, writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		Controls,
		ControlButton,
		SvelteFlowProvider
	} from '@xyflow/svelte'
	import {
		graphBuilder,
		isTriggerStep,
		type AssetN,
		type InlineScript,
		type InsertKind,
		type NodeLayout,
		type onSelectedIteration,
		type SimplifiableFlow
	} from './graphBuilder.svelte'
	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/BranchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
	import { sugiyama, dagStratify, coordCenter, decrossTwoLayer, decrossOpt } from 'd3-dag'
	import { Expand } from 'lucide-svelte'
	import Toggle from '../Toggle.svelte'
	import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	import { clone, encodeState, readFieldsRecursively } from '$lib/utils'
	import BranchOneStart from './renderers/nodes/BranchOneStart.svelte'
	import NoBranchNode from './renderers/nodes/NoBranchNode.svelte'
	import HiddenBaseEdge from './renderers/edges/HiddenBaseEdge.svelte'
	import TriggersNode from './renderers/nodes/TriggersNode.svelte'
	import { Alert, Drawer } from '../common'
	import Button from '../common/button/Button.svelte'
	import FlowYamlEditor from '../flows/header/FlowYamlEditor.svelte'
	import BranchOneEndNode from './renderers/nodes/branchOneEndNode.svelte'
	import type { TriggerContext } from '../triggers'
	import { workspaceStore } from '$lib/stores'
	import SubflowBound from './renderers/nodes/SubflowBound.svelte'
	import { deepEqual } from 'fast-equals'
	import ViewportResizer from './ViewportResizer.svelte'
	import AssetNode from './renderers/nodes/AssetNode.svelte'
	import type { FlowGraphAssetContext } from '../flows/types'
	import { getAllModules } from '../flows/flowExplorer'
	import { inferAssets } from '$lib/infer'
	import OnChange from '../common/OnChange.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import {
		assetDisplaysAsInputInFlowGraph,
		assetDisplaysAsOutputInFlowGraph,
		NODE_WITH_READ_ASSET_Y_OFFSET,
		NODE_WITH_WRITE_ASSET_Y_OFFSET,
		READ_ASSET_Y_OFFSET,
		WRITE_ASSET_Y_OFFSET
	} from '../flows/utils'
	import { assetEq } from '../assets/lib'

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)

	const triggerContext = getContext<TriggerContext>('TriggerContext')

	let fullWidth = 0
	let width = $state(0)

	let simplifiableFlow: SimplifiableFlow | undefined = $state(undefined)

	interface Props {
		success?: boolean | undefined
		modules?: FlowModule[] | undefined
		failureModule?: FlowModule | undefined
		preprocessorModule?: FlowModule | undefined
		minHeight?: number
		maxHeight?: number | undefined
		notSelectable?: boolean
		flowModuleStates?: Record<string, GraphModuleState> | undefined
		selectedId?: Writable<string | undefined>
		path?: string | undefined
		newFlow?: boolean
		insertable?: boolean
		earlyStop?: boolean
		cache?: boolean
		scroll?: boolean
		moving?: string | undefined
		// Download: display a top level button to open the graph in a new tab
		download?: boolean
		fullSize?: boolean
		disableAi?: boolean
		triggerNode?: boolean
		workspace?: string
		editMode?: boolean
		allowSimplifiedPoll?: boolean
		expandedSubflows?: Record<string, FlowModule[]>
		onDelete?: (id: string) => void
		onInsert?: (detail: {
			sourceId?: string
			targetId?: string
			branch?: { rootId: string; branch: number }
			index: number
			detail: string
			isPreprocessor?: boolean
			inlineScript?: InlineScript
			script?: { path: string; summary: string; hash: string | undefined }
			flow?: { path: string; summary: string }
			kind: InsertKind
		}) => Promise<void>
		onNewBranch?: (id: string) => Promise<void>
		onSelect?: (id: string | FlowModule) => void
		onDeleteBranch?: (detail: { id: string; index: number }) => Promise<void>
		onChangeId?: (detail: { id: string; newId: string; deps: Record<string, string[]> }) => void
		onMove?: (id: string) => void
		onUpdateMock?: (detail: { mock: FlowModule['mock']; id: string }) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onSelectedIteration?: onSelectedIteration
		onEditInput?: (moduleId: string, key: string) => void
	}

	let {
		onInsert = undefined,
		onDelete = undefined,
		onMove = undefined,
		onDeleteBranch = undefined,
		onNewBranch = undefined,
		onSelect = undefined,
		onChangeId = undefined,

		onUpdateMock = undefined,
		onSelectedIteration = undefined,
		success = undefined,
		modules = [],
		failureModule = undefined,
		preprocessorModule = undefined,
		minHeight = 0,
		maxHeight = undefined,
		notSelectable = false,
		flowModuleStates = undefined,
		selectedId = writable<string | undefined>(undefined),
		path = undefined,
		newFlow = false,
		insertable = false,
		earlyStop = false,
		cache = false,
		scroll = false,
		moving = undefined,
		download = false,
		fullSize = false,
		disableAi = false,
		triggerNode = false,
		workspace = $workspaceStore ?? 'NO_WORKSPACE',
		editMode = false,
		allowSimplifiedPoll = true,
		expandedSubflows = $bindable({}),
		onTestUpTo = undefined,
		onEditInput = undefined
	}: Props = $props()

	setContext<{
		selectedId: Writable<string | undefined>
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext', { selectedId, useDataflow })

	if (triggerContext && allowSimplifiedPoll) {
		if (isSimplifiable(modules)) {
			triggerContext?.simplifiedPoll?.set(true)
		}
		triggerContext?.simplifiedPoll.subscribe((value) => {
			computeSimplifiableFlow(modules ?? [], value ?? false)
		})
	}

	const flowGraphAssetsCtx: FlowGraphAssetContext = $state({
		val: {
			assetsMap: {},
			selectedAsset: undefined,
			dbManagerDrawer: undefined,
			s3FilePicker: undefined,
			resourceEditorDrawer: undefined,
			resourceMetadataCache: {}
		}
	})
	setContext<FlowGraphAssetContext>('FlowGraphAssetContext', flowGraphAssetsCtx)
	const assetsMap = $derived(flowGraphAssetsCtx.val.assetsMap)

	// Fetch resource metadata for the ExploreAssetButton
	const resMetadataCache = $derived(flowGraphAssetsCtx.val.resourceMetadataCache)
	$effect(() => {
		for (const asset of Object.values(assetsMap ?? []).flatMap((x) => x)) {
			if (asset.kind !== 'resource' || asset.path in resMetadataCache) continue
			resMetadataCache[asset.path] = undefined // avoid fetching multiple times because of async
			ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! }).then(
				(r) => (resMetadataCache[asset.path] = { resource_type: r.resource_type })
			)
		}
	})

	// Fetch transitive assets (path scripts and flows)
	$effect(() => {
		if (!$workspaceStore) return
		let usages: { path: string; kind: AssetUsageKind }[] = []
		let modIds: string[] = []
		for (const mod of getAllModules(modules)) {
			if (mod.id in assetsMap) continue
			assetsMap[mod.id] = [] // avoid fetching multiple times because of async
			if (mod.value.type === 'flow' || mod.value.type === 'script') {
				usages.push({ path: mod.value.path, kind: mod.value.type })
				modIds.push(mod.id)
			}
		}
		if (usages.length) {
			AssetService.listAssetsByUsage({
				workspace: $workspaceStore,
				requestBody: { usages }
			}).then((result) => {
				result.forEach((assets, idx) => {
					assetsMap[modIds[idx]] = assets
				})
			})
		}
	})

	// Prune assetsMap to only contain assets that are actually used
	$effect(() => {
		const allModules = new Set(getAllModules(modules).map((mod) => mod.id))
		for (const modId in assetsMap) {
			if (!allModules.has(modId)) delete assetsMap[modId]
		}
	})

	function computeSimplifiableFlow(modules: FlowModule[], simplifiedFlow: boolean) {
		const isSimplif = isSimplifiable(modules)
		simplifiableFlow = isSimplif ? { simplifiedFlow } : undefined
	}

	onDestroy(() => {
		if (isSimplifiable(modules)) {
			triggerContext?.simplifiedPoll?.set(undefined)
		}
	})

	function onModulesChange(modules: FlowModule[]) {
		computeSimplifiableFlow(
			modules,
			triggerContext?.simplifiedPoll ? (get(triggerContext.simplifiedPoll) ?? false) : false
		)
	}

	let lastNodes: [NodeLayout[], Node[]] | undefined = undefined
	function layoutNodes(nodes: NodeLayout[]): Node[] {
		let lastResult = lastNodes?.[1]
		if (lastResult && nodes === lastNodes?.[0]) {
			return lastResult
		}
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		let nodeWidths: Record<string, number> = {}
		const nodes2 = nodes.map((n) => {
			return { ...n, position: { x: 0, y: 0 } }
		})
		for (const n of nodes.reverse()) {
			const endId = n.id + '-end'

			if (nodeWidths[endId] != undefined) {
				nodeWidths[n.id] = Math.max(nodeWidths[n.id] ?? 0, nodeWidths[endId])
			}
			if (n.parentIds && n.parentIds?.length == 1) {
				const parent = n.parentIds[0]
				const nodeWidth = nodeWidths[n.id] ?? 1
				nodeWidths[parent] = (nodeWidths[parent] ?? 0) + nodeWidth
			}
		}

		const dag = dagStratify().id(({ id }: Node) => id)(nodes2)

		let boxSize: any
		try {
			const layout = sugiyama()
				.decross(nodes.length > 20 ? decrossTwoLayer() : decrossOpt())
				.coord(coordCenter())
				.nodeSize((d) => {
					return [
						(nodeWidths[d?.data?.['id'] ?? ''] ?? 1) * (NODE.width + NODE.gap.horizontal * 1),
						NODE.height + NODE.gap.vertical
					] as readonly [number, number]
				})
			boxSize = layout(dag as any)
		} catch {
			const layout = sugiyama()
				.decross(decrossTwoLayer())
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag as any)
		}

		const yOffset = insertable ? 100 : 0
		const newNodes = dag.descendants().map((des) => ({
			...des.data,
			id: des.data.id,
			position: {
				x: des.x
					? // @ts-ignore
						(des.data.data.offset ?? 0) +
						// @ts-ignore
						des.x +
						(fullSize ? fullWidth : width) / 2 -
						boxSize.width / 2 -
						NODE.width / 2 -
						(width - fullWidth) / 2
					: 0,
				y: (des.y || 0) + yOffset
			}
		}))

		lastNodes = [nodes, newNodes]
		return newNodes
	}

	let computeAssetNodesCache:
		| [Node[], typeof assetsMap, ReturnType<typeof computeAssetNodes>]
		| undefined
	function computeAssetNodes(nodes: Node[], edges: Edge[]): [Node[], Edge[]] {
		if (nodes === computeAssetNodesCache?.[0] && deepEqual(assetsMap, computeAssetNodesCache?.[1]))
			return computeAssetNodesCache[2]

		const ASSET_X_GAP = 20
		const ASSET_WIDTH = 180

		const allAssetNodes: (Node & AssetN)[] = []
		const allAssetEdges: Edge[] = []

		const yPosMap: Record<number, { r?: true; w?: true }> = {}

		for (const node of nodes) {
			const assets = assetsMap?.[node.id]
			let [inputAssetIdx, outputAssetIdx] = [-1, -1]
			let [inputAssetCount, outputAssetCount] = [
				assets?.filter(assetDisplaysAsInputInFlowGraph).length ?? 0,
				assets?.filter(assetDisplaysAsOutputInFlowGraph).length ?? 0
			]

			if (inputAssetCount || outputAssetCount)
				yPosMap[node.position.y] = yPosMap[node.position.y] ?? {}
			if (inputAssetCount) yPosMap[node.position.y].r = true
			if (outputAssetCount) yPosMap[node.position.y].w = true

			// Each asset can be displayed once (R or W) or twice (RW) per node hence the flatMap
			const assetNodes: (Node & AssetN)[] | undefined = assets?.flatMap((asset) => {
				const displayAsInput = assetDisplaysAsInputInFlowGraph(asset)
				const displayAsOutput = assetDisplaysAsOutputInFlowGraph(asset)
				if (displayAsInput) inputAssetIdx++
				if (displayAsOutput) outputAssetIdx++

				const base = { type: 'asset' as const, parentId: node.id, width: ASSET_WIDTH }
				return [
					...(displayAsInput
						? [
								{
									...base,
									data: { asset, displayedAs: 'input' as const },
									id: `${node.id}-asset-in-${asset.kind}-${asset.path}`,
									position: {
										x:
											inputAssetCount === 1
												? (NODE.width - ASSET_WIDTH) / 2 - 10 // Ensure we see the edge
												: (ASSET_WIDTH + ASSET_X_GAP) * (inputAssetIdx - inputAssetCount / 2) +
													(NODE.width + ASSET_X_GAP) / 2,
										y: READ_ASSET_Y_OFFSET
									}
								}
							]
						: []),
					...(displayAsOutput
						? [
								{
									...base,
									data: { asset, displayedAs: 'output' as const },
									id: `${node.id}-asset-out-${asset.kind}-${asset.path}`,
									position: {
										x:
											outputAssetCount === 1
												? (NODE.width - ASSET_WIDTH) / 2 - 10 // Ensure we see the edge
												: (ASSET_WIDTH + ASSET_X_GAP) * (outputAssetIdx - outputAssetCount / 2) +
													(NODE.width + ASSET_X_GAP) / 2,
										y: WRITE_ASSET_Y_OFFSET
									}
								}
							]
						: [])
				]
			})

			const assetEdges = assetNodes?.map((n) => {
				const source = (n.data.displayedAs === 'output' ? n.parentId : n.id) ?? ''
				const target = (n.data.displayedAs === 'output' ? n.id : n.parentId) ?? ''
				return {
					id: `${n.id}-edge`,
					source,
					target,
					type: 'empty',
					data: {
						insertable: false,
						sourceId: source,
						targetId: target,
						moving: moving,
						eventHandlers: eventHandler,
						index: 0,
						enableTrigger: false,
						disableAi: disableAi,
						disableMoveIds: []
					}
				} satisfies Edge
			})

			allAssetEdges.push(...(assetEdges ?? []))
			allAssetNodes.push(...(assetNodes ?? []))
		}

		// Shift all nodes to make space for the new asset nodes
		const sortedNewNodes = clone(nodes.sort((a, b) => a.position.y - b.position.y))
		let currentYOffset = 0
		let prevYPos = NaN
		for (const node of sortedNewNodes) {
			if (node.position.y !== prevYPos) {
				if (yPosMap[prevYPos]?.w) currentYOffset += NODE_WITH_WRITE_ASSET_Y_OFFSET
				if (yPosMap[node.position.y]?.r) currentYOffset += NODE_WITH_READ_ASSET_Y_OFFSET
				prevYPos = node.position.y
			}
			node.position.y += currentYOffset
		}

		let ret: ReturnType<typeof computeAssetNodes> = [
			[...sortedNewNodes, ...allAssetNodes],
			[...edges, ...allAssetEdges]
		]
		computeAssetNodesCache = [nodes, clone(assetsMap), ret]
		return ret
	}

	let eventHandler = {
		deleteBranch: (detail, label) => {
			$selectedId = label
			onDeleteBranch?.(detail)
		},
		insert: (detail) => {
			onInsert?.(detail)
		},
		select: (modId) => {
			if (!notSelectable) {
				if ($selectedId != modId) {
					$selectedId = modId
				}
				onSelect?.(modId)
			}
		},
		changeId: (detail) => {
			onChangeId?.(detail)
		},
		delete: (detail) => {
			onDelete?.(detail.id)
		},
		newBranch: (id) => {
			onNewBranch?.(id)
		},
		move: (detail) => {
			onMove?.(detail.id)
		},
		selectedIteration: (detail) => {
			onSelectedIteration?.(detail)
		},
		simplifyFlow: (detail) => {
			triggerContext?.simplifiedPoll.set(detail)
		},
		expandSubflow: async (id: string, path: string) => {
			const flow = await FlowService.getFlowByPath({ workspace: workspace, path })
			expandedSubflows[id] = flow.value.modules
			expandedSubflows = expandedSubflows
		},
		minimizeSubflow: (id: string) => {
			delete expandedSubflows[id]
			expandedSubflows = expandedSubflows
		},
		updateMock: (detail) => {
			onUpdateMock?.(detail)
		},
		testUpTo: (id: string) => {
			onTestUpTo?.(id)
		},
		editInput: (moduleId: string, key: string) => {
			onEditInput?.(moduleId, key)
		}
	}

	let lastModules = structuredClone($state.snapshot(modules))
	let moduleCounter = $state(0)
	function onModulesChange2(modules) {
		if (!deepEqual(modules, lastModules)) {
			console.log('modules changed', modules)
			lastModules = structuredClone($state.snapshot(modules))
			moduleCounter++
		}
	}

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])

	let height = $state(0)

	function isSimplifiable(modules: FlowModule[] | undefined): boolean {
		if (!modules || modules?.length !== 2) {
			return false
		}
		if (isTriggerStep(modules?.[0])) {
			let secondValue = modules?.[1].value
			return secondValue.type == 'forloopflow'
		}

		return false
	}

	async function updateStores() {
		if (graph.error) {
			return
		}
		let newGraph = graph
		;[nodes, edges] = computeAssetNodes(layoutNodes(newGraph.nodes), newGraph.edges)
		await tick()
		height = Math.max(...nodes.map((n) => n.position.y + NODE.height + 100), minHeight)
	}

	const nodeTypes = {
		input2: InputNode,
		module: ModuleNode,
		branchAllStart: BranchAllStart,
		branchAllEnd: BranchAllEndNode,
		forLoopEnd: ForLoopEndNode,
		forLoopStart: ForLoopStartNode,
		result: ResultNode,
		whileLoopStart: ForLoopStartNode,
		whileLoopEnd: ForLoopEndNode,
		branchOneStart: BranchOneStart,
		branchOneEnd: BranchOneEndNode,
		subflowBound: SubflowBound,
		noBranch: NoBranchNode,
		trigger: TriggersNode,
		asset: AssetNode
	} as any

	const edgeTypes = {
		edge: BaseEdge,
		empty: EmptyEdge,
		dataflowedge: DataflowEdge,
		hiddenedge: HiddenBaseEdge
	} as any

	const proOptions = { hideAttribution: true }

	// onMount(() => {
	// 	centerViewport(width)
	// })
	let yamlEditorDrawer: Drawer | undefined = $state(undefined)

	$effect(() => {
		allowSimplifiedPoll && modules && untrack(() => onModulesChange(modules ?? []))
	})
	$effect(() => {
		modules && untrack(() => onModulesChange2(modules))
	})
	let graph = $derived.by(() => {
		moduleCounter
		return graphBuilder(
			untrack(() => modules),
			{
				disableAi,
				insertable,
				flowModuleStates,
				selectedId: $selectedId,
				path,
				newFlow,
				cache,
				earlyStop,
				editMode
			},
			failureModule,
			preprocessorModule,
			eventHandler,
			success,
			$useDataflow,
			$selectedId,
			moving,
			simplifiableFlow,
			triggerNode ? path : undefined,
			expandedSubflows
		)
	})
	$effect(() => {
		;[graph, allowSimplifiedPoll]
		readFieldsRecursively(assetsMap)
		untrack(() => updateStores())
	})

	let showDataflow = $derived(
		$selectedId != undefined &&
			!$selectedId.startsWith('constants') &&
			!$selectedId.startsWith('settings') &&
			$selectedId !== 'failure' &&
			$selectedId !== 'preprocessor' &&
			$selectedId !== 'Result' &&
			$selectedId !== 'triggers'
	)
	let debouncedWidth: number | undefined = $state(undefined)
	let timeout: NodeJS.Timeout | undefined = $state(undefined)
	$effect(() => {
		if (!debouncedWidth) {
			return
		}
		if (untrack(() => width) == undefined) {
			width = debouncedWidth
			return
		}
		if (untrack(() => timeout)) {
			clearTimeout(untrack(() => timeout))
		}
		timeout = setTimeout(() => {
			if (debouncedWidth && untrack(() => width) != debouncedWidth) {
				width = debouncedWidth
			}
		}, 10)
	})
</script>

{#if insertable}
	<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
{/if}

<div style={`height: ${height}px; max-height: ${maxHeight}px;`} bind:clientWidth={debouncedWidth}>
	{#if graph?.error}
		<div class="center-center p-2">
			<Alert title="Error parsing the flow" type="error" class="max-w-1/2">
				{graph.error}

				<Button
					color="red"
					size="xs"
					btnClasses="mt-2 w-min"
					on:click={() => yamlEditorDrawer?.openDrawer()}>Open YAML editor</Button
				>
			</Alert>
		</div>
	{:else}
		<SvelteFlowProvider>
			<ViewportResizer {width} />
			<SvelteFlow
				onpaneclick={(e) => {
					document.dispatchEvent(new Event('focus'))
				}}
				{nodes}
				{edges}
				{edgeTypes}
				{nodeTypes}
				{height}
				{width}
				minZoom={0.2}
				maxZoom={1.2}
				connectionLineType={ConnectionLineType.SmoothStep}
				defaultEdgeOptions={{ type: 'smoothstep' }}
				preventScrolling={scroll}
				zoomOnDoubleClick={false}
				elementsSelectable={false}
				{proOptions}
				nodesDraggable={false}
				--background-color={false}
			>
				<div class="absolute inset-0 !bg-surface-secondary"></div>
				<Controls position="top-right" orientation="horizontal" showLock={false}>
					{#if download}
						<ControlButton
							onclick={() => {
								try {
									localStorage.setItem(
										'svelvet',
										encodeState({ modules, failureModule, preprocessorModule })
									)
								} catch (e) {
									console.error('error interacting with local storage', e)
								}
								window.open('/view_graph', '_blank')
							}}
							class="!bg-surface"
						>
							<Expand size="14" />
						</ControlButton>
					{/if}
				</Controls>

				<Controls
					position="top-left"
					orientation="horizontal"
					showLock={false}
					showZoom={false}
					showFitView={false}
					class="!shadow-none"
				>
					{#if showDataflow}
						<Toggle
							value={$useDataflow}
							on:change={() => {
								$useDataflow = !$useDataflow
							}}
							size="xs"
							options={{
								right: 'Dataflow'
							}}
						/>
					{/if}
				</Controls>
			</SvelteFlow>
		</SvelteFlowProvider>
	{/if}
</div>

{#each getAllModules(modules) as mod (mod.id)}
	{#if mod.value.type === 'rawscript'}
		{@const v = mod.value}
		<OnChange
			key={[v.content, v.asset_fallback_access_types]}
			runFirstEffect
			onChange={() =>
				inferAssets(v.language, v.content)
					.then((assets) => {
						for (const override of v.asset_fallback_access_types ?? []) {
							assets = assets.map((asset) => {
								if (assetEq(asset, override) && !asset.access_type)
									return { ...asset, access_type: override.access_type }
								return asset
							})
						}
						if (assetsMap && !deepEqual(assetsMap[mod.id], assets)) assetsMap[mod.id] = assets
					})
					.catch((e) => {})}
		/>
	{/if}
{/each}

<S3FilePicker bind:this={flowGraphAssetsCtx.val.s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={flowGraphAssetsCtx.val.dbManagerDrawer} />
<ResourceEditorDrawer bind:this={flowGraphAssetsCtx.val.resourceEditorDrawer} />

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

	:global(.svelte-flow__edgelabel-renderer) {
		@apply z-50;
	}
</style>

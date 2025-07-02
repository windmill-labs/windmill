<script lang="ts">
	import { FlowService, ResourceService, type FlowModule } from '../../gen'
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
	import { formatAsset, parseAsset } from '../assets/lib'
	import type { FlowGraphAssetContext } from '../flows/types'
	import { getAllModules } from '../flows/flowExplorer'
	import { inferAssets } from '$lib/infer'
	import OnChange from '../common/OnChange.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import { NODE_WITH_READ_ASSET_Y_OFFSET, NODE_WITH_WRITE_ASSET_Y_OFFSET } from '../flows/utils'

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
	const resMetadataCache = $derived(flowGraphAssetsCtx.val.resourceMetadataCache)
	$effect(() => {
		for (const { asset } of Object.values(assetsMap ?? []).flatMap((x) => x)) {
			if (asset.kind !== 'resource' || asset.path in resMetadataCache) continue
			ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
				.then((r) => (resMetadataCache[asset.path] = { resourceType: r.resource_type }))
				.catch((err) => (resMetadataCache[asset.path] = undefined))
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

	let lastNodes: [NodeLayout[], Node[], assetsMap: any] | undefined = undefined
	function layoutNodes(nodes: NodeLayout[]): Node[] {
		let lastResult = lastNodes?.[1]
		if (lastResult && deepEqual(nodes, lastNodes?.[0]) && deepEqual(assetsMap, lastNodes?.[2])) {
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
					const id: string | undefined = d?.data?.['id'] ?? ''
					const assetOffsetTop = assetsMap?.[id]?.length ? NODE_WITH_WRITE_ASSET_Y_OFFSET : 0
					return [
						(nodeWidths[id] ?? 1) * (NODE.width + NODE.gap.horizontal * 1),
						NODE.height + NODE.gap.vertical + assetOffsetTop
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

		lastNodes = [nodes, newNodes, clone(assetsMap)]
		return newNodes
	}

	function computeAssetNodes(nodes: Node[], edges: Edge[]): [Node[], Edge[]] {
		const ASSET_X_GAP = 20
		const ASSET_WIDTH = 180
		const READ_ASSET_Y_OFFSET = -45
		const WRITE_ASSET_Y_OFFSET = 58

		const allAssetNodes: (Node & AssetN)[] = []
		const allAssetEdges: Edge[] = []
		const newNodes = [...nodes]

		// If node at yPosition 310.5 has asset nodes on the top, every node
		// at the same yPosition will need to get shifted by the same amount for everything
		// to align
		const yPosAccessTypeMap: Record<number, 'read' | 'write' | 'rw'> = {}

		for (const node of newNodes) {
			const assets = assetsMap?.[node.id]
			const assetNodes: (Node & AssetN)[] | undefined = assets?.map(
				({ asset, accessType }, assetIdx) =>
					({
						id: `${node.id}-asset-${formatAsset(asset)}`,
						type: 'asset',
						data: { asset, accessType },
						position: {
							x:
								(ASSET_WIDTH + ASSET_X_GAP) * (assetIdx - assets.length / 2) +
								(NODE.width + ASSET_X_GAP) / 2,
							y: accessType === 'read' ? READ_ASSET_Y_OFFSET : WRITE_ASSET_Y_OFFSET
						},
						parentId: node.id,
						width: ASSET_WIDTH
					}) satisfies Node & AssetN
			)

			if (assetNodes?.length) {
				if (assetNodes.every((n) => n.data.accessType === 'read')) {
					yPosAccessTypeMap[node.position.y] = 'read'
				} else if (assetNodes.every((n) => n.data.accessType === 'write')) {
					yPosAccessTypeMap[node.position.y] = 'write'
				} else {
					yPosAccessTypeMap[node.position.y] = 'rw'
				}
			}

			const assetEdges = assetNodes?.map((n) => {
				const source = (n.data.accessType !== 'read' ? n.parentId : n.id) ?? ''
				const target = (n.data.accessType !== 'read' ? n.id : n.parentId) ?? ''
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

		// Fix y positions of all nodes that were shifted by layoutNodes
		for (const node of newNodes) {
			if (node.position.y in yPosAccessTypeMap) {
				const accessType = yPosAccessTypeMap[node.position.y]
				if (accessType === 'read') {
					node.position.y = node.position.y + NODE_WITH_READ_ASSET_Y_OFFSET / 2
				} else if (accessType === 'write') {
					node.position.y = node.position.y - NODE_WITH_WRITE_ASSET_Y_OFFSET / 2
				}
			}
		}

		return [
			[...newNodes, ...allAssetNodes],
			[...edges, ...allAssetEdges]
		]
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
			key={v.content}
			runFirstEffect
			onChange={() =>
				inferAssets(v.language, v.content).then((assetsRaw) => {
					const newAssets = assetsRaw.map(parseAsset).filter((a) => !!a)
					if (assetsMap && !deepEqual(assetsMap[mod.id], newAssets))
						assetsMap[mod.id] = newAssets.map((asset) => ({ asset, accessType: 'read' }))
				})}
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

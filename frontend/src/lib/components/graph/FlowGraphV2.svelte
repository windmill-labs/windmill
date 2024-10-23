<script lang="ts">
	import { type FlowModule } from '../../gen'
	import { NODE, type GraphModuleState } from '.'
	import { createEventDispatcher, onMount, setContext } from 'svelte'

	import { writable, type Writable } from 'svelte/store'
	import '@xyflow/svelte/dist/style.css'
	import type { FlowInput } from '../flows/types'
	import {
		SvelteFlow,
		type Node,
		type Edge,
		ConnectionLineType,
		Controls,
		ControlButton,
		type Viewport
	} from '@xyflow/svelte'
	import graphBuilder from './graphBuilder'
	import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	import InputNode from './renderers/nodes/InputNode.svelte'
	import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	import BranchAllEndNode from './renderers/nodes/branchAllEndNode.svelte'
	import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	import ResultNode from './renderers/nodes/ResultNode.svelte'
	import BaseEdge from './renderers/edges/BaseEdge.svelte'
	import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { Expand } from 'lucide-svelte'
	import Toggle from '../Toggle.svelte'
	import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	import { encodeState } from '$lib/utils'
	import BranchOneStart from './renderers/nodes/BranchOneStart.svelte'
	import NoBranchNode from './renderers/nodes/NoBranchNode.svelte'
	import HiddenBaseEdge from './renderers/edges/HiddenBaseEdge.svelte'
	import TriggersNode from './renderers/nodes/TriggersNode.svelte'
	import { Alert, Drawer } from '../common'
	import Button from '../common/button/Button.svelte'
	import FlowYamlEditor from '../flows/header/FlowYamlEditor.svelte'
	import BranchOneEndNode from './renderers/nodes/branchOneEndNode.svelte'
	export let success: boolean | undefined = undefined
	export let modules: FlowModule[] | undefined = []
	export let failureModule: FlowModule | undefined = undefined
	export let preprocessorModule: FlowModule | undefined = undefined
	export let minHeight: number = 0
	export let maxHeight: number | undefined = undefined
	export let notSelectable = false
	export let flowModuleStates: Record<string, GraphModuleState> | undefined = undefined

	export let selectedId: Writable<string | undefined> = writable<string | undefined>(undefined)
	export let path: string | undefined = undefined
	export let newFlow: boolean = false

	export let insertable = false
	export let scroll = false
	export let moving: string | undefined = undefined

	// Download: display a top level button to open the graph in a new tab
	export let download = false
	export let fullSize = false
	export let disableAi = false
	export let flowInputsStore: Writable<FlowInput | undefined> = writable<FlowInput | undefined>(
		undefined
	)
	export let triggerNode = false

	let useDataflow: Writable<boolean | undefined> = writable<boolean | undefined>(false)

	setContext<{
		selectedId: Writable<string | undefined>
		flowInputsStore: Writable<FlowInput | undefined>
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext', { selectedId, flowInputsStore, useDataflow })

	const dispatch = createEventDispatcher()

	let fullWidth = 0
	let width = 0

	function layoutNodes(nodes: Node[]): Node[] {
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}

		const flattenParentIds = nodes.map((n) => ({
			...n,
			parentIds: n.data?.parentIds ?? []
		})) as any

		const stratify = dagStratify().id(({ id }: Node) => id)
		const dag = stratify(flattenParentIds)

		let boxSize: any
		try {
			const layout = sugiyama()
				.decross(decrossOpt())
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag)
		} catch {
			const layout = sugiyama()
				.coord(coordCenter())
				.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
			boxSize = layout(dag)
		}

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
				y: des.y || 0
			}
		}))

		return newNodes
	}

	$: graph = graphBuilder(
		modules,
		{
			disableAi,
			insertable,
			flowModuleStates,
			selectedId: $selectedId,
			path,
			newFlow
		},
		failureModule,
		preprocessorModule,
		{
			deleteBranch: (detail, label) => {
				$selectedId = label

				dispatch('deleteBranch', detail)
			},
			insert: (detail) => {
				dispatch('insert', detail)
			},
			select: (modId) => {
				if (!notSelectable) {
					if ($selectedId != modId) {
						$selectedId = modId
					}
					dispatch('select', modId)
				}
			},
			changeId: (detail) => {
				dispatch('changeId', detail)
			},
			delete: (detail, label) => {
				$selectedId = label

				dispatch('delete', detail)
			},
			newBranch: (module) => {
				dispatch('newBranch', { module })
			},
			move: (module, modules) => {
				dispatch('move', { module, modules })
			},
			selectedIteration: (detail, moduleId) => {
				dispatch('selectedIteration', { ...detail, moduleId: moduleId })
			}
		},
		success,
		$useDataflow,
		$selectedId,
		moving,
		triggerNode
			? {
					path
			  }
			: undefined
	)

	const nodes = writable<Node[]>([])
	const edges = writable<Edge[]>([])

	let height = 0

	function updateStores() {
		if (graph.error) {
			return
		}

		$nodes = layoutNodes(graph?.nodes)
		$edges = graph.edges

		height = Math.max(...$nodes.map((n) => n.position.y + NODE.height + 40), minHeight)
	}

	$: graph && updateStores()

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
		noBranch: NoBranchNode,
		trigger: TriggersNode
	} as any

	const edgeTypes = {
		edge: BaseEdge,
		empty: EmptyEdge,
		dataflowedge: DataflowEdge,
		hiddenedge: HiddenBaseEdge
	} as any

	const proOptions = { hideAttribution: true }

	$: showDataflow =
		$selectedId != undefined &&
		!$selectedId.startsWith('constants') &&
		!$selectedId.startsWith('settings') &&
		$selectedId !== 'failure' &&
		$selectedId !== 'preprocessor' &&
		$selectedId !== 'Result' &&
		$selectedId !== 'triggers'

	const viewport = writable<Viewport>({
		x: 0,
		y: 35,
		zoom: 1
	})

	function centerViewport(width: number) {
		viewport.update((vp) => ({
			...vp,
			x: (width - fullWidth) / 2,
			y: vp.y
		}))
	}

	$: width && centerViewport(width)

	onMount(() => {
		centerViewport(width)
	})
	let yamlEditorDrawer: Drawer | undefined = undefined
</script>

{#if insertable}
	<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
{/if}

<div style={`height: ${height}px; max-height: ${maxHeight}px;`} bind:clientWidth={width}>
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
		<SvelteFlow
			on:paneclick={(e) => {
				document.dispatchEvent(new Event('focus'))
			}}
			{nodes}
			{edges}
			{edgeTypes}
			{nodeTypes}
			{viewport}
			{height}
			minZoom={0.5}
			connectionLineType={ConnectionLineType.SmoothStep}
			defaultEdgeOptions={{ type: 'smoothstep' }}
			preventScrolling={scroll}
			zoomOnDoubleClick={false}
			elementsSelectable={false}
			{proOptions}
			nodesDraggable={false}
			--background-color={false}
		>
			<div class="absolute inset-0 !bg-surface-secondary" />
			<Controls position="top-right" orientation="horizontal" showLock={false}>
				{#if download}
					<ControlButton
						on:click={() => {
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
	{/if}
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

	:global(.svelte-flow__edgelabel-renderer) {
		@apply z-50;
	}
</style>

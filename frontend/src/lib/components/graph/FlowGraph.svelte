<script lang="ts">
	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { type FlowModule, FlowStatusModule } from '../../gen'
	import {
		NODE,
		createIdGenerator,
		isNode,
		isLoop,
		isBranch,
		type GraphItem,
		type Node,
		type Loop,
		type Branch,
		type NestedNodes,
		type GraphModuleState
	} from '.'
	import { defaultIfEmptyString, truncateRev } from '$lib/utils'
	import { createEventDispatcher, setContext } from 'svelte'
	import Svelvet from './svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from './svelvet/types'
	import MapItem from '../flows/map/MapItem.svelte'
	import VirtualItem from '../flows/map/VirtualItem.svelte'
	import { writable, type Writable } from 'svelte/store'

	export let success: boolean | undefined = undefined
	export let modules: FlowModule[] | undefined = []
	export let failureModule: FlowModule | undefined = undefined
	export let minHeight: number = 0
	export let maxHeight: number | undefined = undefined
	export let notSelectable = false
	export let flowModuleStates: Record<string, GraphModuleState> | undefined = undefined
	export let rebuildOnChange: any = undefined

	export let selectedId: Writable<string | undefined> = writable<string | undefined>(undefined)

	export let insertable = false
	export let moving: string | undefined = undefined
	export let scroll = false

	setContext<{ selectedId: Writable<string | undefined> }>('FlowGraphContext', { selectedId })

	let idGenerator: Generator
	let nestedNodes: NestedNodes
	let nodes: Node[] = []
	let edges: UserEdgeType[] = []
	let width: number, height: number

	let errorHandlers: Record<string, string> = {}

	let dispatch = createEventDispatcher()

	$: {
		rebuildOnChange
		moving
		width && height && minHeight && $selectedId && flowModuleStates
		nodes = edges = []
		errorHandlers = {}
		createGraph()
	}

	async function createGraph() {
		if (modules) {
			idGenerator = createIdGenerator()
		} else {
			nodes = edges = []
			return
		}
		nestedNodes = nodes = []

		nestedNodes.push(
			createVirtualNode(
				getParentIds(),
				'Input',
				modules,
				'after',
				undefined,
				undefined,
				0,
				0,
				true,
				undefined
			)
		)

		modules.forEach((m, i) => {
			const item = getConvertedFlowModule(
				m,
				undefined,
				undefined,
				0,
				i + 1 == modules?.length,
				modules!
			)
			item && nestedNodes.push(item)
		})
		nestedNodes.push(
			createVirtualNode(
				getParentIds(),
				'Result',
				undefined,
				'before',
				undefined,
				undefined,
				0,
				modules.length,
				true,
				undefined
			)
		)

		if (!flowModuleStates) {
			if (failureModule) nestedNodes.push(createErrorHandler(failureModule))
		} else {
			Object.entries(flowModuleStates ?? [])
				.filter(([k, v]) => k.startsWith('failure'))
				.forEach(([k, v]) => {
					nestedNodes.push(createErrorHandler({ id: k } as FlowModule, v.parent_module))
				})
		}
		const flatNodes = flattenNestedNodes(nestedNodes)

		const layered = layoutNodes(flatNodes)

		nodes = layered.nodes
		// width = layered.width
		height = Math.min(Math.max(layered.height, minHeight), maxHeight ?? window.innerHeight - 100)
		edges = createEdges(nodes)
	}

	function getConvertedFlowModule(
		module: FlowModule,
		parent: NestedNodes | string | undefined,
		edgeLabel: string | undefined,
		loopDepth: number,
		insertableEnd: boolean,
		modules: FlowModule[]
	): GraphItem | undefined {
		const type = module.value.type
		const parentIds = getParentIds(parent)
		if (type === 'forloopflow') {
			//@ts-ignore
			return flowModuleToLoop(modules, module, parent, loopDepth)
		} else if (type === 'branchone') {
			const branches = [
				{ summary: 'Default Branch', modules: module.value.default, removable: false },
				...module.value.branches.map((b, i) => ({
					summary: defaultIfEmptyString(b.summary, 'Branch ' + (i + 1)),
					modules: b.modules,
					removable: true
				}))
			]
			return flowModuleToBranch(
				module,
				modules,
				branches,
				['', ...module.value.branches.map((x) => `${truncateRev(x.expr, 20)}`)],
				parent,
				loopDepth,
				false
			)
		} else if (type === 'branchall') {
			const branches = module.value.branches.map((b, i) => ({
				summary: defaultIfEmptyString(b.summary, `Branch ${i + 1}`),
				modules: b.modules,
				removable: true
			}))
			return flowModuleToBranch(module, modules, branches, [], parent, loopDepth, true)
		}
		return flowModuleToNode(
			parentIds,
			module,
			edgeLabel,
			undefined,
			loopDepth,
			insertableEnd,
			false,
			modules
		)
	}

	function getParentIds(items: string | NestedNodes | undefined = undefined): string[] {
		if (typeof items == 'string') {
			return [items]
		}
		const item = items?.at(-1) || nestedNodes.at(-1)
		if (!item) return []

		if (isNode(item)) {
			const id = item.id
			return [id]
		} else if (isLoop(item)) {
			return getParentIds(item.items)
		} else if (isBranch(item)) {
			return [item.nodeEnd.id]
		}
		return []
	}

	function getStateColor(state: FlowStatusModule.type | undefined): string {
		switch (state) {
			case FlowStatusModule.type.SUCCESS:
				return 'rgb(193, 255, 216)'
			case FlowStatusModule.type.FAILURE:
				return 'rgb(248 113 113)'
			case FlowStatusModule.type.IN_PROGRESS:
				return 'rgb(253, 240, 176)'
			case FlowStatusModule.type.WAITING_FOR_EVENTS:
				return 'rgb(229, 176, 253)'
			case FlowStatusModule.type.WAITING_FOR_EXECUTOR:
				return 'rgb(255, 208, 193)'
			default:
				return '#fff'
		}
	}

	function getResultColor(): string {
		switch (success) {
			case true:
				return getStateColor(FlowStatusModule.type.SUCCESS)
			case false:
				return getStateColor(FlowStatusModule.type.FAILURE)
			default:
				return '#fff'
		}
	}
	function flowModuleToNode(
		parentIds: string[],
		mod: FlowModule,
		edgeLabel: string | undefined,
		annotation: string | undefined,
		loopDepth: number,
		insertableEnd: boolean,
		branchable: boolean,
		modules: FlowModule[]
	): Node {
		return {
			type: 'node',
			id: mod.id,
			position: { x: -1, y: -1 },
			data: {
				custom: {
					component: MapItem,
					props: {
						trigger: parentIds.length == 0,
						mod,
						insertable,
						insertableEnd,
						branchable,
						bgColor: getStateColor(flowModuleStates?.[mod.id]?.type),
						// annotation,
						modules,
						moving
					},
					cb: (e: string, detail: any) => {
						if (e == 'delete') {
							dispatch('delete', detail)
						} else if (e == 'select') {
							if (!notSelectable) {
								if ($selectedId != mod.id) {
									$selectedId = mod.id
									dispatch('select', mod)
								}
							}
						} else if (e == 'insert') {
							dispatch('insert', detail)
						} else if (e == 'newBranch') {
							dispatch('newBranch', detail)
						} else if (e == 'move') {
							dispatch('move', { module: mod, modules })
						}
					}
				}
			},
			width: NODE.width,
			height: NODE.height,
			parentIds,
			sourcePosition: 'bottom',
			targetPosition: 'top',
			edgeLabel,
			loopDepth
		}
	}

	function flowModuleToLoop(
		modules: FlowModule[],
		module: FlowModule & { value: { type: 'forloopflow' } },
		parent: NestedNodes | string | undefined,
		loopDepth: number
	): Loop {
		const loop: Loop = {
			type: 'loop',
			items: [
				flowModuleToNode(
					getParentIds(parent),
					module,
					undefined,
					flowModuleStates?.[module.id]?.iteration_total
						? 'Iteration ' + flowModuleStates?.[module.id]?.iteration_total
						: '',
					loopDepth,
					false,
					false,
					modules
				)
			]
		}
		const innerModules = module.value.modules
		loop.items.push(
			createVirtualNode(
				getParentIds(loop.items),
				`Do one iteration`,
				innerModules,
				'after',
				undefined,
				1000,
				loopDepth + 1,
				0,
				true,
				undefined,
				undefined
			)
		)
		innerModules.forEach((module, i) => {
			const item = getConvertedFlowModule(
				module,
				loop.items,
				undefined,
				loopDepth + 1,
				i + 1 == innerModules?.length,
				innerModules
			)
			item && loop.items.push(item)
		})
		loop.items.push(
			createVirtualNode(
				getParentIds(loop.items),
				`Collect result of each iteration`,
				modules,
				'after',
				undefined,
				1000,
				loopDepth,
				modules.findIndex((m) => m.id == module.id) + 1,
				true,
				undefined,
				module.id
			)
		)
		return loop
	}

	function flowModuleToBranch(
		module: FlowModule,
		modules: FlowModule[],
		branches: { summary: string; modules: FlowModule[]; removable: boolean }[],
		edgesLabel: string[],
		parent: string | NestedNodes | undefined = undefined,
		loopDepth: number,
		branchall: boolean
	): Branch | Node {
		const node = flowModuleToNode(
			getParentIds(parent),
			module,
			undefined,
			undefined,
			loopDepth,
			false,
			true,
			modules
		)
		const bitems: NestedNodes[] = []
		const branchParent = [node.id]
		if (branches.length == 0) {
			bitems.push([
				createVirtualNode(
					branchParent,
					'No branches',
					undefined,
					'after',
					undefined,
					0,
					loopDepth,
					0,
					false,
					undefined
				)
			])
		}

		branches.forEach(({ summary, modules, removable }, i) => {
			const items: NestedNodes = []
			items.push(
				createVirtualNode(
					branchParent,
					summary,
					modules,
					'after',
					edgesLabel[i],
					undefined,
					loopDepth,
					0,
					false,
					removable ? { module, index: i } : undefined
				)
			)
			if (modules.length) {
				modules.forEach((module, j) => {
					const item = getConvertedFlowModule(
						module,
						items,
						undefined,
						loopDepth,
						j + 1 == modules?.length,
						modules
					)
					item && items.push(item)
				})
			}
			items.length && bitems.push(items)
		})

		return {
			type: 'branch',
			node,
			nodeEnd: createVirtualNode(
				bitems.map((i) => getParentIds(i)).flat(),
				branchall ? 'Collect result of each branch' : 'Result of the chosen branch',
				modules,
				'after',
				undefined,
				0,
				loopDepth,
				modules.findIndex((m) => m.id == module.id) + 1,
				true,
				undefined,
				module.id
			),
			items: bitems
		}
	}

	function flattenNestedNodes(nestedNodes: NestedNodes, nodes: Node[] = []): Node[] {
		const array = nodes
		nestedNodes.forEach((node) => {
			if (isNode(node)) {
				array.push(node)
			} else if (isLoop(node)) {
				flattenNestedNodes(node.items, array)
			} else if (isBranch(node)) {
				array.push(node.node)
				node.items.forEach((item) => {
					flattenNestedNodes(item, array)
				})
				array.push(node.nodeEnd)
			}
		})
		return array
	}

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number } {
		const stratify = dagStratify().id(({ id }: Node) => id)
		const dag = stratify(nodes)

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
		return {
			nodes: dag.descendants().map((des) => ({
				...des.data,
				id: des.data.id,
				position: {
					x: des.x
						? des.data.loopDepth * 50 + des.x + width / 2 - boxSize.width / 2 - NODE.width / 2
						: 0,
					y: des.y || 0
				}
			})),
			height: boxSize.height + NODE.height
		}
	}

	function createEdges(nodes: Node[]): UserEdgeType[] {
		const edges: UserEdgeType[] = []
		nodes.forEach((node) => {
			node.parentIds.forEach((pid, i) => {
				// skip virtual nodes such as collect result
				if (
					false &&
					errorHandlers[pid] &&
					parseInt(node.id) < 900 &&
					nodes.find((x) => x.id == errorHandlers[pid])
				) {
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: pid,
						target: errorHandlers[pid],
						labelBgColor: 'white',
						arrow: false,
						animate: false,
						noHandle: true,
						label: node.edgeLabel,
						type: 'bezier'
					})
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: errorHandlers[pid],
						target: node.id,
						labelBgColor: 'white',
						arrow: false,
						animate: false,
						noHandle: true,
						label: node.edgeLabel,
						type: 'bezier'
					})
				} else {
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: pid,
						target: node.id,
						labelBgColor: 'white',
						arrow: false,
						animate: false,
						noHandle: true,
						label: node.edgeLabel,
						edgeColor: '#999',
						type: 'bezier'
					})
				}
			})
		})
		return edges
	}

	function createVirtualNode(
		parentIds: string[],
		label: string,
		modules: FlowModule[] | undefined,
		whereInsert: 'before' | 'after' | undefined,
		edgeLabel: string | undefined,
		offset: number | undefined,
		loopDepth: number,
		index: number,
		selectable: boolean,
		deleteBranch: { module: FlowModule; index: number } | undefined,
		mid: string | undefined = undefined
	): Node {
		const id = -idGenerator.next().value - 2 + (offset ?? 0)
		return {
			type: 'node',
			id: id.toString(),
			position: { x: -1, y: -1 },
			data: {
				// html: `
				// 	<div class="w-full max-h-full text-center ellipsize-multi-line text-2xs [-webkit-line-clamp:2] px-1">
				// 		${label}
				// 	</div>
				// `
				custom: {
					component: VirtualItem,
					props: {
						label,
						insertable,
						modules,
						bgColor: label == 'Result' ? getResultColor() : '#dfe6ee',
						selected: $selectedId == label,
						index,
						selectable,
						whereInsert,
						deleteBranch,
						id: mid,
						moving
					},
					cb: (e: string, detail: any) => {
						if (e == 'insert') {
							dispatch('insert', detail)
						} else if (e == 'select') {
							$selectedId = detail
							dispatch('select', detail)
						} else if (e == 'deleteBranch') {
							$selectedId = label
							dispatch('deleteBranch', detail)
						}
					}
				}
			},
			width: NODE.width,
			height: NODE.height,
			borderColor: $selectedId == label ? 'black' : '#999',
			sourcePosition: 'bottom',
			targetPosition: 'top',
			parentIds,
			edgeLabel,
			loopDepth
		}
	}

	function createErrorHandler(mod: FlowModule, parent_module?: string): Node {
		const nId = (-idGenerator.next().value - 1 + 1100).toString()
		parent_module && (errorHandlers[parent_module] = nId)
		let label = 'Error handler'
		return {
			type: 'node',
			id: nId,
			position: { x: -1, y: -1 },
			data: {
				custom: {
					component: VirtualItem,
					props: {
						label,
						insertable: false,
						modules: undefined,
						bgColor: getStateColor(flowModuleStates?.[mod.id]?.type),
						selected: $selectedId == mod.id,
						index: 0,
						selectable: true,
						id: mod.id
					},
					cb: (e: string, detail: any) => {
						console.log(detail)
						if (e == 'select') {
							$selectedId = detail
							dispatch('select', detail)
						}
					}
				}
			},
			width: NODE.width,
			height: NODE.height,
			sourcePosition: 'bottom',
			targetPosition: 'top',
			parentIds: parent_module ? [parent_module] : [],
			loopDepth: 0
		}
	}
</script>

<div bind:clientWidth={width} class="w-full h-full overflow-hidden relative">
	{#if width && height}
		<Svelvet
			highlightEdges={false}
			locked
			{nodes}
			{width}
			{edges}
			{height}
			{scroll}
			background={false}
			bgColor="rgb(249 250 251)"
		/>
	{/if}
</div>

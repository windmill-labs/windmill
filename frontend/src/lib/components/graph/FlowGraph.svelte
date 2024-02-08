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
		type GraphModuleState,
		getStateColor
	} from '.'
	import { defaultIfEmptyString, encodeState } from '$lib/utils'
	import { createEventDispatcher, onMount, setContext } from 'svelte'
	import Svelvet from './svelvet/container/views/Svelvet.svelte'
	import type { UserEdgeType } from './svelvet/types'
	import MapItem from '../flows/map/MapItem.svelte'
	import VirtualItem from '../flows/map/VirtualItem.svelte'
	import { writable, type Writable } from 'svelte/store'
	import { getDependeeAndDependentComponents } from '../flows/flowExplorer'
	import { deepEqual } from 'fast-equals'
	import DarkModeObserver from '../DarkModeObserver.svelte'

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
	export let download = false
	export let fullSize = false
	export let disableAi = false

	setContext<{ selectedId: Writable<string | undefined> }>('FlowGraphContext', { selectedId })

	let idGenerator: Generator
	let nestedNodes: NestedNodes
	let nodes: Node[] = []
	let edges: UserEdgeType[] = []
	let width: number, height: number
	let fullWidth: number
	let errorHandlers: Record<string, string> = {}

	let error: string | undefined = undefined

	$: showDataflow =
		$selectedId != undefined &&
		!$selectedId.startsWith('constants') &&
		!$selectedId.startsWith('settings') &&
		$selectedId !== 'failure' &&
		$selectedId !== 'Result'
	let dataflow = false

	let dispatch = createEventDispatcher()
	let renderCount = 1

	$: {
		dataflow
		moving
		success
		width && height && minHeight && $selectedId && flowModuleStates && renderCount

		createGraph()
	}

	$: rebuildOnChange && triggerRebuild()

	let oldRebuildOnChange = rebuildOnChange ? JSON.parse(JSON.stringify(rebuildOnChange)) : undefined

	function triggerRebuild() {
		if (!deepEqual(oldRebuildOnChange, rebuildOnChange)) {
			oldRebuildOnChange = JSON.parse(JSON.stringify(rebuildOnChange))
			createGraph()
		}
	}

	async function createGraph() {
		try {
			// console.log(JSON.stringify(modules))
			// return
			nodes = []
			edges = []
			errorHandlers = {}

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
					undefined,
					undefined,
					0,
					0,
					true,
					undefined,
					undefined,
					'Input'
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
					undefined,
					undefined,
					0,
					modules.length,
					true,
					undefined,
					undefined,
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

			let hfull = Math.max(layered.height, minHeight)
			fullWidth = layered.width
			height = fullSize ? hfull : Math.min(hfull, maxHeight ?? window.innerHeight * 1.5)

			let useDataflow = dataflow && showDataflow
			edges = useDataflow ? [] : createEdges(nodes)

			if (useDataflow && $selectedId) {
				let deps = getDependeeAndDependentComponents($selectedId, modules ?? [], failureModule)
				if (deps) {
					Object.entries(deps.dependees).forEach((x, i) => {
						let pid = x[0]
						edges.push({
							id: `dep-${pid}-${$selectedId}`,
							source: pid,
							target: $selectedId!,
							labelBgColor: 'white',
							arrow: false,
							animate: true,
							noHandle: true,
							label: pid,
							type: 'bezier',
							offset: i * 20
						})
					})

					Object.entries(deps.dependents).forEach((x, i) => {
						let pid = x[0]
						edges.push({
							id: `dep-${pid}-${$selectedId}`,
							source: $selectedId!,
							target: pid,
							labelBgColor: 'white',
							arrow: false,
							animate: true,
							noHandle: true,
							label: pid,
							type: 'bezier',
							offset: i * 10
						})
					})
				}
			}
			if (error) error = undefined
		} catch (e) {
			error = e.message
		}
	}

	function getConvertedFlowModule(
		module: FlowModule,
		parent: NestedNodes | string | undefined,
		edgeLabel: string | undefined,
		loopDepth: number,
		insertableEnd: boolean,
		modules: FlowModule[],
		wrapper: FlowModule | undefined = undefined
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
					summary: defaultIfEmptyString(b.summary, 'Branch ' + (i + 1)) + '\n`' + b.expr + '`',
					modules: b.modules,
					removable: true
				}))
			]
			return flowModuleToBranch(
				module,
				modules,
				branches,
				[], //['', ...module.value.branches.map((x) => `${truncateRev(x.expr, 20)}`)],
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
			modules,
			wrapper
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

	function getResultColor(): string {
		const isDark = document.documentElement.classList.contains('dark')

		switch (success) {
			case true:
				return getStateColor(FlowStatusModule.type.SUCCESS)
			case false:
				return getStateColor(FlowStatusModule.type.FAILURE)
			default:
				return isDark ? '#2e3440' : '#fff'
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
		modules: FlowModule[],
		wrapper: FlowModule | undefined = undefined
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
						duration_ms: flowModuleStates?.[mod.id]?.duration_ms,
						bgColor: getStateColor(flowModuleStates?.[mod.id]?.type),
						annotation,
						modules,
						moving,
						disableAi,
						wrapperId: wrapper?.id
					},
					cb: (e: string, detail: any) => {
						if (e == 'delete') {
							dispatch('delete', detail)
						} else if (e == 'select') {
							if (!notSelectable) {
								if ($selectedId != mod.id) {
									$selectedId = mod.id
								}
								dispatch('select', mod)
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
					flowModuleStates?.[module.id]?.iteration
						? 'Iteration ' +
								flowModuleStates?.[module.id]?.iteration +
								'/' +
								(flowModuleStates?.[module.id]?.iteration_total ?? '?')
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
				undefined,
				1000,
				loopDepth + 1,
				0,
				false,
				undefined,
				undefined,
				undefined,
				true,
				module
			)
		)
		innerModules.forEach((innerModule, i) => {
			const item = getConvertedFlowModule(
				innerModule,
				loop.items,
				undefined,
				loopDepth + 1,
				i + 1 == innerModules?.length,
				innerModules,
				module
			)
			item && loop.items.push(item)
		})
		loop.items.push(
			createVirtualNode(
				getParentIds(loop.items),
				`Collect result of each iteration`,
				modules,
				undefined,
				1000,
				loopDepth,
				modules.findIndex((m) => m.id == module.id) + 1,
				true,
				undefined,
				module.id,
				undefined
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
		const wrapper = JSON.parse(JSON.stringify(module))
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
					undefined,
					0,
					loopDepth,
					0,
					false,
					undefined,
					undefined,
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
					edgesLabel[i],
					undefined,
					loopDepth,
					0,
					false,
					removable ? { module, index: i } : undefined,
					undefined,
					undefined,
					false,
					wrapper
				)
			)
			if (modules.length) {
				modules.forEach((innerModule, j) => {
					const item = getConvertedFlowModule(
						innerModule,
						items,
						undefined,
						loopDepth,
						j + 1 == modules?.length,
						modules,
						module
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
				undefined,
				0,
				loopDepth,
				modules.findIndex((m) => m.id == module.id) + 1,
				true,
				undefined,
				module.id,
				undefined
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

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number; width: number } {
		let seenId: string[] = []
		for (const n of nodes) {
			if (seenId.includes(n.id)) {
				n.id = n.id + '_dup'
			}
			seenId.push(n.id)
		}
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
						? des.data.loopDepth * 50 +
						  des.x +
						  (fullSize ? fullWidth : width) / 2 -
						  boxSize.width / 2 -
						  NODE.width / 2
						: 0,
					y: des.y || 0
				}
			})),
			height: boxSize.height + NODE.height,
			width: boxSize.width + NODE.width
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
		edgeLabel: string | undefined,
		offset: number | undefined,
		loopDepth: number,
		index: number,
		selectable: boolean,
		deleteBranch: { module: FlowModule; index: number } | undefined,
		mid: string | undefined,
		fixed_id: string | undefined,
		center: boolean = true,
		wrapperNode: FlowModule | undefined = undefined
	): Node {
		const id = fixed_id ?? -idGenerator.next().value - 2 + (offset ?? 0)

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
						bgColor:
							label == 'Result'
								? getResultColor()
								: document.documentElement.classList.contains('dark')
								? '#2e3440'
								: '#dfe6ee',
						selected: $selectedId == label,
						index,
						selectable,
						deleteBranch,
						id: mid,
						moving,
						center,
						disableAi,
						wrapperNode
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
						id: mod.id,
						disableAi
					},
					cb: (e: string, detail: any) => {
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

	function onThemeChange() {
		renderCount++
	}

	onMount(() => {
		onThemeChange()
	})
</script>

<DarkModeObserver on:change={onThemeChange} />

<!-- {JSON.stringify(flowModuleStates)} -->
<div
	bind:clientWidth={width}
	class={fullSize ? '' : 'w-full h-full overflow-hidden relative'}
	id="flow-graph"
>
	{#if !error}
		{#if width && height}
			{#key renderCount}
				<Svelvet
					on:expand={() => {
						try {
							localStorage.setItem('svelvet', encodeState({ modules, failureModule }))
						} catch (e) {
							console.error('error interacting with local storage', e)
						}
						window.open('/view_graph', '_blank')
					}}
					{download}
					highlightEdges={false}
					locked
					bind:dataflow
					{nodes}
					width={fullSize ? fullWidth : width}
					{edges}
					{height}
					{scroll}
					nodeSelected={showDataflow}
					background={false}
				/>
			{/key}
		{/if}
	{:else}
		<div class="p-4 text-center text-xl text-red-600">
			Error computing the graph: {error}
		</div>
	{/if}
</div>

<script lang="ts">
	import Svelvet from '@windmill-labs/svelvet'

	import { sugiyama, dagStratify, decrossOpt, coordCenter } from 'd3-dag'
	import { type FlowModule, FlowStatusModule, type ForloopFlow, RawScript } from '../../gen'
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
		type ModuleHost,
		type GraphModuleState,
		type Edge
	} from '.'
	import { defaultIfEmptyString, truncateRev } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { charsToNumber, numberToChars } from '../flows/utils'

	export let modules: FlowModule[] | undefined = []
	export let failureModule: FlowModule | undefined = undefined
	export let minHeight: number = 0
	export let notSelectable = false
	export let flowModuleStates: Record<string, GraphModuleState> | undefined = undefined

	let selectedNode: string | undefined = undefined

	let idGenerator: Generator
	let nestedNodes: NestedNodes
	let nodes: Node[] = []
	let edges: Edge[] = []
	let width: number, height: number

	let errorHandlers: Record<string, number> = {}

	let dispatch = createEventDispatcher()

	$: {
		width && height && minHeight && selectedNode && flowModuleStates
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

		nestedNodes.push(createVirtualNode(getParentIds(), 'Input'))

		modules.forEach((m) => {
			const item = getConvertedFlowModule(m)
			item && nestedNodes.push(item)
		})
		nestedNodes.push(createVirtualNode(getParentIds(), 'Result'))

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
		height = layered.height
		edges = createEdges(nodes)
	}

	function getConvertedFlowModule(
		module: FlowModule,
		parent: NestedNodes | string | undefined = undefined,
		edgeLabel: string | undefined = undefined,
		insideLoop: boolean = false
	): GraphItem | undefined {
		const type = module.value.type
		const parentIds = getParentIds(parent)
		if (type === 'rawscript') {
			const lang = module.value.language
			return flowModuleToNode(
				parentIds,
				module.id,
				module.summary || 'Inline script',
				'inline',
				module,
				lang,
				edgeLabel,
				undefined,
				insideLoop
			)
		} else if (type === 'script') {
			const isHub = module.value.path.startsWith('hub/')
			return flowModuleToNode(
				parentIds,
				module.id,
				module.summary || module.value.path,
				isHub ? 'hub' : 'workspace',
				module,
				undefined,
				edgeLabel,
				undefined,
				insideLoop
			)
		} else if (type === 'forloopflow') {
			return flowModuleToLoop(module.value.modules, module, parent)
		} else if (type === 'branchone') {
			const branches = [module.value.default, ...module.value.branches.map((b) => b.modules)]
			return flowModuleToBranch(
				module,
				branches,
				['Default', ...module.value.branches.map((x) => `If ${truncateRev(x.expr, 20)}`)],
				parent,
				insideLoop
			)
		} else if (type === 'branchall') {
			const branches = module.value.branches.map((b) => b.modules)
			return flowModuleToBranch(module, branches, [], parent, insideLoop)
		} else if (type === 'flow') {
			return flowModuleToNode(
				parentIds,
				module.id,
				module.summary || 'Flow ' + module.value.path,
				'inline',
				module,
				undefined,
				edgeLabel,
				undefined,
				insideLoop
			)
		}
		return flowModuleToNode(
			parentIds,
			module.id,
			module.summary || 'Identity step',
			'inline',
			module,
			undefined,
			edgeLabel,
			undefined,
			insideLoop
		)
	}

	function getParentIds(items: string | NestedNodes | undefined = undefined): string[] {
		if (typeof items == 'string') {
			return [items]
		}
		const item = items?.at(-1) || nestedNodes.at(-1)
		if (!item) return []

		if (isNode(item)) {
			const id = numberToChars(item.id)
			return [id]
		} else if (isLoop(item)) {
			return getParentIds(item.items)
		} else if (isBranch(item)) {
			return item.items.map((i) => getParentIds(i)).flat()
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
	function flowModuleToNode(
		parentIds: string[],
		id: string,
		title: string,
		host: ModuleHost,
		onClickDetail: any,
		lang?: RawScript.language,
		edgeLabel?: string,
		header?: string,
		insideLoop: boolean = false
	): Node {
		const langImg: Record<RawScript.language, string> = {
			deno: '/icons/ts-lang.svg',
			go: '/icons/go-lang.svg',
			python3: '/icons/python-lang.svg',
			bash: '/icons/bash-lang.svg'
		}
		const hostImg: Record<ModuleHost, string> = {
			hub: '/icons/hub-script.svg',
			workspace: '/icons/inline-script.svg',
			inline: ''
		}
		const wrapperWidth = lang ? 'w-[calc(100%-70px)]' : 'w-[calc(100%-50px)]'
		let nodeId = id ?? numberToChars(idGenerator.next().value - 1)

		return {
			id: charsToNumber(nodeId),
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full flex justify-between items-center px-1">
					<div class="${wrapperWidth} text-left ellipsize text-2xs truncate">
						${title}
					</div>
					<div class="flex items-center">
						${lang ? `<img src="${langImg[lang]}" class="grayscale">` : ''}
						${host != 'inline' ? `<img src="${hostImg[host]}" class="grayscale">` : ''}
						<span class="center-center font-semibold bg-indigo-100 text-indigo-800 rounded px-1 pb-[2px] ml-[2px]">
							${nodeId}
						</span>
					</div>
				</div>
				<div class="text-2xs absolute -top-6 text-gray-600 truncate">${
					flowModuleStates?.[nodeId]?.scheduled_for ?? header ?? ''
				}<div>
			`
			},
			host,
			width: insideLoop ? NODE.width * 0.8 : NODE.width,
			height: NODE.height,
			borderColor: selectedNode == nodeId ? 'black' : '#999',
			bgColor: selectedNode == nodeId ? '#f5f5f5' : getStateColor(flowModuleStates?.[nodeId]?.type),
			parentIds,
			clickCallback: (node) => {
				if (!notSelectable) {
					selectedNode = nodeId
				}
				if (onClickDetail.id == undefined) {
					onClickDetail.id = nodeId
				}
				dispatch('click', onClickDetail)
			},
			edgeLabel
		}
	}

	function flowModuleToLoop(
		modules: FlowModule[],
		module: FlowModule,
		parent: NestedNodes | string | undefined = undefined
	): Loop {
		const value = module.value as ForloopFlow
		const expr = value.iterator.type == 'static' ? value.iterator.value : value.iterator.expr
		const loop: Loop = {
			type: 'loop',
			items: [
				flowModuleToNode(
					getParentIds(parent),
					module.id,
					module.summary || `For Loop: ${defaultIfEmptyString(expr ?? '', 'TBD')}`,
					'inline',
					module,
					undefined,
					undefined,
					flowModuleStates?.[module.id]?.iteration_total
						? 'Iteration ' + flowModuleStates?.[module.id]?.iteration_total
						: ''
				)
			]
		}
		modules.forEach((module) => {
			const item = getConvertedFlowModule(module, loop.items, undefined, true)
			item && loop.items.push(item)
		})
		loop.items.push(
			createVirtualNode(
				getParentIds(loop.items),
				`Collect iterations' results of For Loop ${module.id ?? ''}`,
				undefined,
				1000
			)
		)
		return loop
	}

	function flowModuleToBranch(
		module: FlowModule,
		branches: FlowModule[][],
		edgesLabel: string[],
		parent: string | NestedNodes | undefined = undefined,
		insideLoop: boolean = false
	): Branch {
		const branch: Branch = {
			type: 'branch',
			node: flowModuleToNode(
				getParentIds(parent),
				module.id,
				module.summary || module.value.type == 'branchall'
					? 'Run all branches'
					: 'Run one branch given predicate',
				'inline',
				module,
				undefined,
				undefined,
				undefined,
				insideLoop
			),
			items: []
		}
		const branchParent = [numberToChars(branch.node.id)]
		if (branches.length == 0) {
			branch.items.push([createVirtualNode(branchParent, 'No branches')])
		}
		branches.forEach((modules, i) => {
			const items: NestedNodes = []
			if (!modules.length) {
				items.push(createVirtualNode(branchParent, 'Empty branch', edgesLabel[i]))
			} else {
				modules.forEach((module) => {
					const item = getConvertedFlowModule(
						module,
						items.length ? items : numberToChars(branch.node.id),
						edgesLabel[i],
						insideLoop
					)
					item && items.push(item)
				})
			}
			items.length && branch.items.push(items)
		})
		return branch
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
			}
		})
		return array
	}

	function layoutNodes(nodes: Node[]): { nodes: Node[]; height: number } {
		const stratify = dagStratify().id(({ id }: Node) => numberToChars(id))
		const dag = stratify(nodes)

		const layout = sugiyama()
			.decross(decrossOpt())
			.coord(coordCenter())
			.nodeSize(() => [NODE.width + NODE.gap.horizontal, NODE.height + NODE.gap.vertical])
		const boxSize = layout(dag)
		return {
			nodes: dag.descendants().map((des) => ({
				...des.data,
				id: des.data.id,
				position: {
					x: des.x ? des.x + (width - boxSize.width - NODE.width) / 2 : 0,
					y: des.y || 0
				}
			})),
			height: Math.max(boxSize.height + NODE.height, minHeight)
		}
	}

	function createEdges(nodes: Node[]): Edge[] {
		const edges: Edge[] = []
		nodes.forEach((node) => {
			node.parentIds.forEach((pid, i) => {
				// skip virtual nodes such as collect result
				if (errorHandlers[pid] && node.id < 900 && nodes.find((x) => x.id == errorHandlers[pid])) {
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: charsToNumber(pid),
						target: errorHandlers[pid],
						labelBgColor: 'white',
						arrow: true,
						animate: false,
						noHandle: false,
						label: node.edgeLabel
						// type: 'smoothstep'
					})
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: errorHandlers[pid],
						target: node.id,
						labelBgColor: 'white',
						arrow: true,
						animate: false,
						noHandle: false,
						label: node.edgeLabel
						// type: 'smoothstep'
					})
				} else {
					edges.push({
						id: `e-${pid}-${node.id}`,
						source: charsToNumber(pid),
						target: node.id,
						labelBgColor: 'white',
						arrow: true,
						animate: false,
						noHandle: false,
						label: node.edgeLabel
						// type: 'smoothstep'
					})
				}
			})
		})
		return edges
	}

	function createVirtualNode(
		parentIds: string[],
		label: string,
		edgesLabel?: string,
		offset?: number
	): Node {
		return {
			id: -idGenerator.next().value - 1 + (offset ?? 0),
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full max-h-full text-center ellipsize-multi-line text-2xs [-webkit-line-clamp:2] px-1">
					${label}
				</div>
			`
			},
			width: NODE.width,
			height: NODE.height,
			borderColor: selectedNode == label ? 'black' : '#999',
			bgColor: selectedNode == label ? '#f5f5f5' : '#d4e4ff',
			parentIds,
			clickCallback: (node) => {
				if (!notSelectable) {
					selectedNode = label
				}
				dispatch('click', label)
			}
		}
	}

	function createErrorHandler(mod: FlowModule, parent_module?: string): Node {
		const nId = -idGenerator.next().value - 1 + 1100
		parent_module && (errorHandlers[parent_module] = nId)
		return {
			id: nId,
			position: { x: -1, y: -1 },
			data: {
				html: `
				<div class="w-full flex justify-between items-center px-1">
					<div class="text-left ellipsize text-2xs truncate">
						Error Handler
					</div>
					<div class="flex items-center">
						<span class="center-center font-semibold bg-indigo-100 text-indigo-800 rounded px-1 pb-[2px] ml-[2px]">
							${mod.id}
						</span>
					</div>
				</div>
			`
			},
			width: NODE.width,
			height: NODE.height,
			bgColor: selectedNode == mod.id ? '#f5f5f5' : getStateColor(flowModuleStates?.[mod.id]?.type),
			borderColor: selectedNode == mod.id ? 'black' : '#999',
			parentIds: parent_module ? [parent_module] : [],
			clickCallback: (node) => {
				if (!notSelectable) {
					selectedNode = mod.id
				}
				dispatch('click', mod)
			}
		}
	}
</script>

<div bind:clientWidth={width} class="w-full h-full overflow-hidden relative">
	{#if width && height}
		<Svelvet {nodes} {width} {edges} {height} bgColor="rgb(249 250 251)" />
	{/if}
</div>

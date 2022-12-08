import type { FlowStatusModule } from "$lib/gen"
import type { Node as SvelvetNode } from "svelvet"

export type ModuleHost = 'workspace' | 'inline' | 'hub'

export type Node = SvelvetNode & { parentIds: string[], edgeLabel?: string, host?: ModuleHost }

export type Loop = {
	type: 'loop',
	items: NestedNodes
}

export type Branch = {
	node: Node,
	type: 'branch',
	items: NestedNodes[]
}

export type GraphItem = Node | Loop | Branch

export type GraphModuleState = {
	type: FlowStatusModule.type
	logs?: string
	result?: any
	scheduled_for?: string
	job_id?: string
	parent_module?: string
	iteration_total?: number
	retries?: number
}

export type NestedNodes = GraphItem[]

export function isNode(item: GraphItem | NestedNodes | undefined): item is Node {
	return typeof item?.['id'] === 'number'
}

export function isLoop(item: GraphItem | NestedNodes | undefined): item is Loop {
	return item?.['type'] === 'loop'
}

export function isBranch(item: GraphItem | NestedNodes | undefined): item is Branch {
	return item?.['type'] === 'branch'
}
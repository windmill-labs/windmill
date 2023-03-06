import type { FlowStatusModule } from "$lib/gen"
import type { UserNodeType } from "./svelvet/types";

export type ModuleHost = 'workspace' | 'inline' | 'hub'

export type Node = UserNodeType & { parentIds: string[], edgeLabel?: string, host?: ModuleHost, type: 'node', loopDepth: number }

export type Loop = {
	type: 'loop',
	items: NestedNodes
}

export type Branch = {
	node: Node,
	nodeEnd: Node,
	type: 'branch',
	items: NestedNodes[]
}

export type GraphItem = Node | Loop | Branch

export type GraphModuleState = {
	type: FlowStatusModule.type
	args: any
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
	return item?.['type'] === 'node'
}

export function isLoop(item: GraphItem | NestedNodes | undefined): item is Loop {
	return item?.['type'] === 'loop'
}

export function isBranch(item: GraphItem | NestedNodes | undefined): item is Branch {
	return item?.['type'] === 'branch'
}
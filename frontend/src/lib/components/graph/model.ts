import type { FlowStatusModule } from '$lib/gen'
import type { Writable } from 'svelte/store'
import type { UserNodeType } from './svelvet/types'
import type { FlowState } from '../flows/flowState'

export type ModuleHost = 'workspace' | 'inline' | 'hub'

export type Node = UserNodeType & {
	parentIds: string[]
	edgeLabel?: string
	host?: ModuleHost
	type: 'node'
	loopDepth: number
}

export type Loop = {
	type: 'loop'
	items: NestedNodes
}

export type Branch = {
	node: Node
	nodeEnd: Node
	type: 'branch'
	items: NestedNodes[]
}

export type GraphItem = Node | Loop | Branch

export type GraphModuleStates = {
	job_id: string
	states: Record<string, GraphModuleState>
}

export type DurationStatus = {
	iteration_from?: number
	iteration_total?: number
	byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
}

export type FlowStatusViewerContext = {
	flowStateStore?: Writable<FlowState>
	retryStatus: Writable<Record<string, number | undefined>>
	suspendStatus: Writable<Record<string, number | undefined>>
}
export type GraphModuleState = {
	type: FlowStatusModule.type
	args: any
	logs?: string
	result?: any
	scheduled_for?: Date
	job_id?: string
	parent_module?: string
	iteration?: number
	iteration_total?: number
	retries?: number
	duration_ms?: number
	started_at?: number
	suspend_count?: number
	isListJob?: boolean
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

import type { FlowStatusModule, Job } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { FlowState } from '../flows/flowState'

export type ModuleHost = 'workspace' | 'inline' | 'hub'

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
	byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
}

export type FlowStatusViewerContext = {
	flowState?: FlowState
	retryStatus: StateStore<Record<string, number | undefined>>
	suspendStatus: StateStore<Record<string, { nb: number; job: Job }>>
	hideDownloadInGraph?: boolean
	hideTimeline?: boolean
	hideNodeDefinition?: boolean
	hideJobId?: boolean
	hideDownloadLogs
}

export type GraphModuleState = {
	type: FlowStatusModule['type']
	args: any
	logs?: string
	flow_jobs_results?: any
	branchChosen?: number
	result?: any
	tag?: string
	scheduled_for?: Date
	job_id?: string
	parent_module?: string
	selectedForloop?: string
	selectedForloopIndex?: number
	selectedForLoopSetManually?: boolean
	flow_jobs_success?: (boolean | undefined)[]
	flow_jobs_duration?: {
		started_at?: (string | undefined)[]
		duration_ms?: (number | undefined)[]
	}
	flow_jobs?: string[]
	iteration_total?: number
	retries?: number
	duration_ms?: number
	started_at?: number
	suspend_count?: number
	isListJob?: boolean
	skipped?: boolean
	agent_actions?: FlowStatusModule['agent_actions']
	script_hash?: string
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

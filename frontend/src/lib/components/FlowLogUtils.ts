import type { FlowStatusModule, Job } from '$lib/gen'

export interface FlowData {
	jobId: string
	inputs: any
	result: any
	logs?: string
	steps: StepData[]
	status: Job['type']
	success?: boolean
}

export interface StepData {
	stepId: string
	stepNumber: number
	summary?: string
	inputs: any
	result?: any
	jobId?: string
	logs?: string
	status: FlowStatusModule['type']
	subflows?: FlowData[]
	selectedIteration?: number
	type:
		| 'script'
		| 'flow'
		| 'identity'
		| 'branchall'
		| 'rawscript'
		| 'forloopflow'
		| 'whileloopflow'
		| 'branchone'
		| undefined
}

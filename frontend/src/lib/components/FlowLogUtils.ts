import type { FlowStatusModule, Job } from '$lib/gen'

export interface FlowData {
	jobId: string
	inputs: any
	result: any
	logs?: string
	steps: StepData[]
	status: Job['type']
	success?: boolean
	label?: string
	emptyFlow?: boolean
	flow_status?: { step?: number }
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
	iterationTotal?: number
	flowJobs?: string[]
	flowJobsSuccess?: (boolean | undefined)[]
	selectedManually?: boolean | undefined
	emptySubflow?: boolean
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

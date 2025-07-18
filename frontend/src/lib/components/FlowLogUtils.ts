export interface FlowData {
	jobId: string
	inputs: any
	result: any
	steps: StepData[]
	status: 'success' | 'failure' | 'in_progress' | 'waiting'
}

export interface StepData {
	stepId: string
	stepNumber: number
	summary?: string
	inputs: any
	result?: any
	jobId?: string
	logs?: string
	status: 'success' | 'failure' | 'in_progress' | 'waiting'
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

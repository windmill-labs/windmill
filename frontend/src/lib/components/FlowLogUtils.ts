export interface FlowData {
	jobId: string
	inputs: any
	result: any
	steps: StepData[]
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
	iterations?: FlowData[]
	selectedIteration?: number
}

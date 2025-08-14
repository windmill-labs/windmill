import type { FlowModuleValue } from '$lib/gen'

export interface FlowLogEntry {
	id: string
	stepId: string
	stepNumber?: number
	summary?: string
	stepType?: FlowModuleValue['type']
	subflows?: FlowLogEntry[][]
	subflowsSummary?: string[]
}

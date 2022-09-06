import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: string
	select: (id: string) => void
	schedule: Schedule
	path: string
}

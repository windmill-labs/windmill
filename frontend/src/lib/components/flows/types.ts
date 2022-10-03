import type { Writable } from 'svelte/store'
import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: Writable<string>
	select: (id: string) => void
	schedule: Writable<Schedule>,
	previewArgs: Writable<Record<string, any>>,
}

import type { Writable } from 'svelte/store'
import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: Writable<string>
	select: (id: string) => void
	schedule: Writable<Schedule>
	path: string
	previewArgs: Writable<Record<string, any>>
}

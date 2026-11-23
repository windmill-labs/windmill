import { writable, type Writable } from 'svelte/store'

export type UiIntent = { kind: 'open_module_tab'; componentId: string; tab: string }

export const uiIntentStore: Writable<UiIntent | null> = writable(null)

/** The id a step's editor registers its intents under: loops and branches each have their own
 *  editor, every other step type is a FlowModuleComponent. */
export function moduleIntentComponentId(type: string | undefined, moduleId: string): string {
	const own = ['forloopflow', 'whileloopflow', 'branchone', 'branchall']
	return `${type && own.includes(type) ? type : 'flow'}-${moduleId}`
}

export function emitUiIntent(intent: UiIntent) {
	uiIntentStore.set(intent)
}

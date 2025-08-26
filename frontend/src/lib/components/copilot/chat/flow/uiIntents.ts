import { writable, type Writable } from 'svelte/store'

export type UiIntent = { kind: 'open_module_tab'; componentId: string; tab: string }

export const uiIntentStore: Writable<UiIntent | null> = writable(null)

export function emitUiIntent(intent: UiIntent) {
	uiIntentStore.set(intent)
}

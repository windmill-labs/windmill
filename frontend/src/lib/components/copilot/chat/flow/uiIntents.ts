import { writable, type Writable } from 'svelte/store'

export type UiIntent =
	| { kind: 'open_module_tab'; id: string; tab: string }
	| { kind: 'highlight_setting'; id: string; section: string; field?: string }

export type UiIntentEnvelope = {
	id: string
	intent: UiIntent
	source: 'ai_tool' | 'user' | 'system'
	once?: boolean // default true: auto-clear after handled
	ts: number
}

export const uiIntentStore: Writable<UiIntentEnvelope | null> = writable(null)

export function emitUiIntent(intent: UiIntent, source: UiIntentEnvelope['source'] = 'ai_tool') {
	uiIntentStore.set({
		id: crypto.randomUUID(),
		intent,
		source,
		once: true,
		ts: Date.now()
	})
}

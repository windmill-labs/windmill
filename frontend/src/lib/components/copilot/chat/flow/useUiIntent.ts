import { uiIntentStore, type UiIntent } from './uiIntents'
import { onDestroy } from 'svelte'

type Handlers = {
	openTab?: (tab: string) => void
}

export function useUiIntent(componentId: string, handlers: Handlers) {
	const unsub = uiIntentStore.subscribe((intent: UiIntent | null) => {
		if (!intent || intent.componentId !== componentId) return

		if (intent.kind === 'open_module_tab') handlers.openTab?.(intent.tab)

		uiIntentStore.set(null)
	})
	onDestroy(() => unsub())
}

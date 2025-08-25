import { uiIntentStore, type UiIntent } from './uiIntents'
import { onDestroy } from 'svelte'

type Handlers = {
	openTab?: (tab: string) => void
	highlight?: (section: string, field?: string) => void
}

export function useUiIntent(componentId: string, handlers: Handlers) {
	const unsub = uiIntentStore.subscribe((intent: UiIntent | null) => {
		if (!intent || intent.componentId !== componentId) return

		if (intent.kind === 'open_module_tab') handlers.openTab?.(intent.tab)
		if (intent.kind === 'highlight_setting') handlers.highlight?.(intent.section, intent.field)

		uiIntentStore.set(null)
	})
	onDestroy(() => unsub())
}

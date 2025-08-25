import { uiIntentStore, type UiIntentEnvelope } from './uiIntents'
import { onMount } from 'svelte'

type Handlers = {
	openTab?: (tab: string) => void
	highlight?: (section: string, field?: string) => void
}

export function useUiIntent(moduleId: string, handlers: Handlers) {
	let lastHandled: string | undefined

	const unsub = uiIntentStore.subscribe((env: UiIntentEnvelope | null) => {
		if (!env || env.id === lastHandled) return
		const i = env.intent
		if ('id' in i && i.id !== moduleId) return

		if (i.kind === 'open_module_tab') handlers.openTab?.(i.tab)
		if (i.kind === 'highlight_setting') handlers.highlight?.(i.section, i.field)

		lastHandled = env.id
		if (env.once !== false) uiIntentStore.set(null)
	})

	onMount(() => () => unsub())
}

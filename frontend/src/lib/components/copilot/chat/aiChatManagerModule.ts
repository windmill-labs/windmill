import { lazyModule } from '$lib/lazyModule.svelte'

/** The chat engine, for the app shell: it wraps every page and reaches the engine only from a
 *  click, and the engine is the largest module graph the shell would otherwise load first.
 *  Pages that drive the chat themselves import `./AIChatManager.svelte` statically. */
export const aiChatManagerModule = lazyModule(() => import('./AIChatManager.svelte'))

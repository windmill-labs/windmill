import { getContext } from 'svelte'
import { aiChatManager as singletonAiChatManager, AIChatManager } from './AIChatManager.svelte'

const AI_CHAT_MANAGER_CONTEXT_KEY = 'aiChatManager'

// Resolve the AIChatManager instance for the current component subtree.
// Callers (e.g. the sessions pane) may set a scoped instance via
// `setContext('aiChatManager', ...)`; everywhere else falls back to the
// app-wide singleton so default usage stays unchanged.
export function getAiChatManager(): AIChatManager {
	return getContext<AIChatManager>(AI_CHAT_MANAGER_CONTEXT_KEY) ?? singletonAiChatManager
}

// Padding profile shared by every chat input textarea / mirror layer.
// `!pr-10` reserves space for the absolute-positioned send/stop button at
// `bottom-1 right-1`; changing the button geometry means updating this
// constant in one place rather than four.
export const CHAT_INPUT_PADDING = '!pl-3 !pr-10 !py-2'

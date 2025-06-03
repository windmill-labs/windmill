// frontend/src/lib/components/copilot/chat/AIChat.svelte.ts

import type { ScriptOptions } from './ContextManager.svelte'
import type { FlowAIChatHelpers } from './flow/core'
type TriggerablesMap = Record<
	string,
	{ description: string; onTrigger: ((id: string) => void) | undefined }
>

export class AIChat {
	triggerablesByAI = $state<TriggerablesMap>({})
	open = $state<boolean>(false)
	initialInput = $state<string>('')
	size = $state<number>(300)
	scriptEditorOptions = $state<ScriptOptions | undefined>(undefined)
	scriptEditorApplyCode = $state<((code: string) => void) | undefined>(undefined)
	scriptEditorShowDiffMode = $state<(() => void) | undefined>(undefined)
	flowAiChatHelpers = $state<FlowAIChatHelpers | undefined>(undefined)
}

export const AIChatService = new AIChat()

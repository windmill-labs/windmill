import { BROWSER } from 'esm-env'

export const triggerablesByAi = $state<TriggerablesMap>({})

export type TriggerablesMap = Record<
	string,
	{
		description: string
		onTrigger: ((value?: string) => void) | undefined
	}
>
export const DEFAULT_SIZE = 22

type ChatState = {
	size: number
	// Whether a docked chat pane is mounted (set by AiChatLayout). False in AI
	// Sessions mode, where the root layout renders no pane: aiChatManager.openChat()
	// only writes `size`, so a button that opens the pane without checking this is a
	// silent no-op. Entry points that can't fall back to a session must hide instead.
	dockedChatAvailable: boolean
}

// we first check BROWSER before localStorage to avoid SSR issues when using Drawer in a SvelteKit app (chatState is imported in Drawer.svelte)
export const chatState = $state<ChatState>({
	size: BROWSER && localStorage.getItem('ai-chat-open') === 'true' ? DEFAULT_SIZE : 0,
	dockedChatAvailable: false
})

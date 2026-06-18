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
}

// we first check BROWSER before localStorage to avoid SSR issues when using Drawer in a SvelteKit app (chatState is imported in Drawer.svelte)
export const chatState = $state<ChatState>({
	size: BROWSER && localStorage.getItem('ai-chat-open') === 'true' ? DEFAULT_SIZE : 0
})

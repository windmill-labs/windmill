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
export const chatState = $state<ChatState>({
    size: localStorage.getItem('ai-chat-open') === 'true' ? DEFAULT_SIZE : 0,
})

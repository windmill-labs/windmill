import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import { createChat } from './chat'
import type { Chat, ChatOptions, ChatState } from './types'

export type UseWindmillChat = ChatState &
  Pick<
    Chat,
    | 'sendMessage'
    | 'stop'
    | 'newConversation'
    | 'selectConversation'
    | 'loadConversations'
    | 'deleteConversation'
    | 'loadOlderMessages'
  > & { chat: Chat }

/**
 * A chat on a chat-mode flow. The chat is created once per `flowPath`, `baseUrl`,
 * `workspace`, `history` and token string, and destroyed on unmount: a new token
 * string is a new user, whose chat must not carry the previous one's state. A
 * token function and the callbacks are read through refs, so passing a new
 * closure on a render changes nothing but what the next call runs.
 */
export function useWindmillChat(options: ChatOptions): UseWindmillChat {
  const latest = useRef(options)
  latest.current = options
  const tokenString = typeof options.token === 'string' ? options.token : undefined
  const chat = useMemo(
    () =>
      createChat({
        ...options,
        token:
          typeof options.token === 'function'
            ? () => {
                const token = latest.current.token
                return typeof token === 'function' ? token() : (token ?? '')
              }
            : options.token,
        onFinish: (turn) => latest.current.onFinish?.(turn),
        onError: (error, turn) => latest.current.onError?.(error, turn)
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [options.flowPath, options.baseUrl, options.workspace, options.history, tokenString]
  )
  useEffect(() => () => chat.destroy(), [chat])
  const state = useSyncExternalStore(chat.subscribe, chat.getState, chat.getState)
  return useMemo(
    () => ({
      ...state,
      chat,
      sendMessage: chat.sendMessage,
      stop: chat.stop,
      newConversation: chat.newConversation,
      selectConversation: chat.selectConversation,
      loadConversations: chat.loadConversations,
      deleteConversation: chat.deleteConversation,
      loadOlderMessages: chat.loadOlderMessages
    }),
    [state, chat]
  )
}

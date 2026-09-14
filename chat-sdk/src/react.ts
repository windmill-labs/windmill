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
 * `workspace`, `history`, `storageKey` and credential, and destroyed on unmount. A
 * credential change is a new user, whose chat must not carry the previous one's
 * state: a different token string, or a switch between no token, a token string
 * and a token function, all recreate it. A token function is read through a ref
 * on every call, so a new closure per render changes what the next call runs and
 * nothing else; pass a `storageKey` per user when local history must not be shared.
 * The callbacks are read through refs too.
 */
export function useWindmillChat(options: ChatOptions): UseWindmillChat {
  const latest = useRef(options)
  latest.current = options
  const credential =
    typeof options.token === 'function' ? 'fn' : typeof options.token === 'string' ? `str:${options.token}` : 'none'
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
    [options.flowPath, options.baseUrl, options.workspace, options.history, options.storageKey, credential]
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

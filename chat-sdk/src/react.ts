import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import { createChat } from './chat'
import type { Chat, ChatOptions, ChatState } from './types'

export type UseWindmillChat = ChatState &
  Pick<
    Chat,
    | 'sendMessage'
    | 'resumeTurn'
    | 'stop'
    | 'newConversation'
    | 'selectConversation'
    | 'loadConversations'
    | 'deleteConversation'
    | 'renameConversation'
    | 'loadOlderMessages'
    | 'refreshMessages'
  > & { chat: Chat }

/**
 * A chat on a chat-mode flow. The chat is created once per `flowPath`, `baseUrl`,
 * `workspace`, `history`, `storageKey`, credential and presence of `run`, and
 * destroyed on unmount. A credential change is a new user, whose chat must not
 * carry the previous one's state: a different token string, or a switch between no
 * token, a token string and a token function, all recreate it. A token function is
 * read through a ref on every call, so a new closure per render changes what the
 * next call runs and nothing else; pass a `storageKey` per user when local history
 * must not be shared. `run`, the callbacks and `inputs` are read the same way: the
 * latest render's values go with the next message.
 */
export function useWindmillChat(options: ChatOptions): UseWindmillChat {
  const latest = useRef(options)
  latest.current = options
  const credential =
    typeof options.token === 'function' ? 'fn' : typeof options.token === 'string' ? `str:${options.token}` : 'none'
  // A custom runner replaces the deployed flow call, so its presence is part of what the chat is.
  const customRun = options.run !== undefined
  const chat = useMemo(
    () =>
      createChat({
        ...options,
        // Sent per message from the latest render instead, so a removed key stays removed.
        inputs: undefined,
        token:
          typeof options.token === 'function'
            ? () => {
                const token = latest.current.token
                return typeof token === 'function' ? token() : (token ?? '')
              }
            : options.token,
        run: customRun ? (args, turn) => (latest.current.run ?? options.run!)(args, turn) : undefined,
        onFinish: (turn) => latest.current.onFinish?.(turn),
        onError: (error, turn) => latest.current.onError?.(error, turn)
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [options.flowPath, options.baseUrl, options.workspace, options.history, options.storageKey, credential, customRun]
  )
  useEffect(() => () => chat.destroy(), [chat])
  const state = useSyncExternalStore(chat.subscribe, chat.getState, chat.getState)
  return useMemo(
    () => ({
      ...state,
      chat,
      sendMessage: (text, options) =>
        chat.sendMessage(text, { ...options, inputs: { ...latest.current.inputs, ...options?.inputs } }),
      resumeTurn: chat.resumeTurn,
      stop: chat.stop,
      newConversation: chat.newConversation,
      selectConversation: chat.selectConversation,
      loadConversations: chat.loadConversations,
      deleteConversation: chat.deleteConversation,
      renameConversation: chat.renameConversation,
      loadOlderMessages: chat.loadOlderMessages,
      refreshMessages: chat.refreshMessages
    }),
    [state, chat]
  )
}

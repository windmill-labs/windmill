import { useEffect, useMemo, useSyncExternalStore } from 'react'
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
 * A chat on a chat-mode flow. The chat is created once per `flowPath` /
 * `baseUrl` / `workspace` / `history` and destroyed on unmount; the other options
 * are read when it is created.
 */
export function useWindmillChat(options: ChatOptions): UseWindmillChat {
  const chat = useMemo(
    () => createChat(options),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [options.flowPath, options.baseUrl, options.workspace, options.history]
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

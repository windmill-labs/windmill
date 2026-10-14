import type { ChatMessage, Conversation, StorageLike } from './types'

export interface LocalHistory {
  listConversations(): Conversation[]
  getMessages(conversationId: string): ChatMessage[]
  upsertConversation(conversation: Conversation): void
  /** Changes a stored conversation's title in place; unlike `upsertConversation`, its position is kept. */
  renameConversation(conversationId: string, title: string): void
  saveMessages(conversationId: string, messages: ChatMessage[]): void
  deleteConversation(conversationId: string): void
}

interface Snapshot {
  v: 1
  conversations: Conversation[]
  messages: Record<string, ChatMessage[]>
}

const MAX_CONVERSATIONS = 100

/**
 * Conversation history in the browser, for a credential shared by every visitor
 * (server history would show them each other's chats). One key per flow, and per
 * `storageKey` when the caller sets one; every operation re-reads the store so
 * several tabs stay consistent.
 */
export function createLocalHistory(storage: StorageLike | undefined, key: string): LocalHistory {
  const store = storage ?? defaultStorage()
  const read = (): Snapshot => {
    try {
      const raw = store.getItem(key)
      if (raw) {
        const parsed = JSON.parse(raw) as Snapshot
        if (parsed && parsed.v === 1) return parsed
      }
    } catch {
      // unreadable: start over
    }
    return { v: 1, conversations: [], messages: {} }
  }
  const write = (snapshot: Snapshot) => {
    try {
      store.setItem(key, JSON.stringify(snapshot))
    } catch {
      // quota or private mode: the page keeps working from memory
    }
  }
  return {
    listConversations: () => read().conversations,
    getMessages: (id) => read().messages[id] ?? [],
    upsertConversation(conversation) {
      const s = read()
      const rest = s.conversations.filter((c) => c.id !== conversation.id)
      s.conversations = [conversation, ...rest]
      for (const dropped of s.conversations.splice(MAX_CONVERSATIONS)) {
        delete s.messages[dropped.id]
      }
      write(s)
    },
    renameConversation(id, title) {
      const s = read()
      s.conversations = s.conversations.map((c) => (c.id === id ? { ...c, title } : c))
      write(s)
    },
    saveMessages(id, messages) {
      const s = read()
      s.messages[id] = messages.map((m) => ({ ...m, pending: false }))
      write(s)
    },
    deleteConversation(id) {
      const s = read()
      s.conversations = s.conversations.filter((c) => c.id !== id)
      delete s.messages[id]
      write(s)
    }
  }
}

function defaultStorage(): StorageLike {
  try {
    const ls = globalThis.localStorage
    if (ls) {
      const probe = '__windmill_chat_probe__'
      ls.setItem(probe, '1')
      ls.removeItem(probe)
      return ls
    }
  } catch {
    // no localStorage (SSR, blocked storage): fall through
  }
  const memory = new Map<string, string>()
  return {
    getItem: (k) => memory.get(k) ?? null,
    setItem: (k, v) => void memory.set(k, v),
    removeItem: (k) => void memory.delete(k)
  }
}

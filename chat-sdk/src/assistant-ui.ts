import {
  useExternalStoreRuntime,
  type AppendMessage,
  type AssistantRuntime,
  type ExternalStoreAdapter,
  type ThreadMessageLike
} from '@assistant-ui/react'
import { useEffect, useMemo } from 'react'
import { useWindmillChat } from './react'
import type { ChatMessage, ChatOptions } from './types'
import { parseJsonOr } from './utils'

/** One assistant turn: the assistant and tool rows between two user messages. */
export interface WindmillTurn {
  id: string
  role: 'user' | 'assistant' | 'system'
  messages: ChatMessage[]
}

export interface WindmillRuntimeOptions extends ChatOptions {
  /** Load the conversation list on mount so `ThreadListPrimitive` has something to show. Default true. */
  threadList?: boolean
}

/**
 * An assistant-ui runtime over a chat-mode flow, for `AssistantRuntimeProvider`.
 * Conversations become threads, so the thread list primitives switch, create and
 * delete Windmill conversations.
 */
export function useWindmillRuntime(options: WindmillRuntimeOptions): AssistantRuntime {
  const chat = useWindmillChat(options)
  const turns = useMemo(() => groupTurns(chat.messages), [chat.messages])
  const withThreadList = options.threadList !== false
  useEffect(() => {
    if (withThreadList) chat.loadConversations().catch(() => {})
  }, [chat.chat, withThreadList])

  const adapter: ExternalStoreAdapter<WindmillTurn> = {
    messages: turns,
    isRunning: chat.status === 'submitted' || chat.status === 'streaming',
    convertMessage: toThreadMessage,
    onNew: async (message: AppendMessage) => {
      if (message.role !== 'user') return
      await chat.sendMessage(appendedText(message))
    },
    onCancel: async () => {
      await chat.stop()
    },
    adapters: withThreadList
      ? {
          threadList: {
            threadId: chat.conversationId,
            threads: chat.conversations.map((c) => ({ status: 'regular' as const, id: c.id, title: c.title })),
            onSwitchToNewThread: () => chat.newConversation(),
            onSwitchToThread: (id) => chat.selectConversation(id),
            onDelete: (id) => chat.deleteConversation(id),
            onRename: (id, title) => chat.renameConversation(id, title)
          }
        }
      : undefined
  }
  return useExternalStoreRuntime<WindmillTurn>(adapter)
}

function appendedText(message: AppendMessage): string {
  return message.content
    .filter((p): p is { type: 'text'; text: string } => p.type === 'text')
    .map((p) => p.text)
    .join('\n')
}

/** Folds the role-per-row message list into turns: one entry per user message, one per answer. */
export function groupTurns(messages: ChatMessage[]): WindmillTurn[] {
  const turns: WindmillTurn[] = []
  for (const m of messages) {
    const last = turns[turns.length - 1]
    if (m.role === 'user' || m.role === 'system' || !last || last.role !== 'assistant') {
      turns.push({ id: m.id, role: m.role === 'tool' ? 'assistant' : m.role, messages: [m] })
    } else {
      last.messages.push(m)
    }
  }
  return turns
}

/** A turn as assistant-ui content parts; the status reflects streaming and a failed flow. */
export function toThreadMessage(turn: WindmillTurn): ThreadMessageLike {
  const first = turn.messages[0]
  const createdAt = new Date(first.createdAt)
  if (turn.role !== 'assistant') {
    return { id: turn.id, role: turn.role, createdAt, content: [{ type: 'text', text: first.content }] }
  }
  const content: ThreadContentPart[] = []
  for (const m of turn.messages) {
    if (m.reasoning) content.push({ type: 'reasoning', text: m.reasoning })
    if (m.role === 'tool') {
      const args = parseJsonOr(m.tool?.arguments)
      content.push({
        type: 'tool-call',
        toolCallId: m.tool?.callId ?? m.id,
        toolName: m.tool?.name ?? 'tool',
        args: (isJsonObject(args) ? args : args === undefined ? {} : { input: args }) as ToolCallArgs,
        argsText: m.tool?.arguments ?? '',
        result:
          m.tool?.status === 'running' ? undefined : m.tool?.result !== undefined ? parseJsonOr(m.tool.result) : m.content,
        isError: m.tool?.status === 'error'
      })
      continue
    }
    if (m.content) content.push({ type: 'text', text: m.content })
  }
  const last = turn.messages[turn.messages.length - 1]
  const status: ThreadMessageLike['status'] = turn.messages.some((m) => m.pending)
    ? { type: 'running' }
    : last.role === 'assistant' && !last.success
      ? { type: 'incomplete', reason: 'error', error: last.content }
      : { type: 'complete', reason: 'stop' }
  return { id: turn.id, role: 'assistant', createdAt, content, status }
}

type ThreadContentPart = Exclude<ThreadMessageLike['content'], string>[number]
type ToolCallArgs = Extract<ThreadContentPart, { type: 'tool-call' }>['args']

function isJsonObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

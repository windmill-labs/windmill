import { describe, expect, it, vi } from 'vitest'
import type { Chat, ChatMessage, ChatState, Conversation } from 'windmill-chat'
import { FlowChatPool } from './flowChatPool'

/** A chat whose state the test drives; `selectConversation` only sets the id. */
function fakeChat() {
	let state: ChatState = {
		conversationId: undefined,
		messages: [],
		status: 'idle',
		error: undefined,
		conversations: [],
		history: 'server',
		loadingMessages: false,
		hasMoreMessages: false
	}
	const listeners = new Set<(s: ChatState) => void>()
	const set = (patch: Partial<ChatState>) => {
		state = { ...state, ...patch }
		for (const listener of listeners) listener(state)
	}
	const chat = {
		getState: () => state,
		subscribe: (listener: (s: ChatState) => void) => {
			listeners.add(listener)
			listener(state)
			return () => listeners.delete(listener)
		},
		sendMessage: vi.fn(async () => {}),
		resumeTurn: vi.fn(async () => {}),
		stop: vi.fn(async () => {}),
		newConversation: vi.fn(),
		selectConversation: vi.fn(async (id: string) => set({ conversationId: id })),
		loadConversations: vi.fn(async () => []),
		deleteConversation: vi.fn(async () => {}),
		renameConversation: vi.fn(async () => {}),
		loadOlderMessages: vi.fn(async () => {}),
		destroy: vi.fn()
	} satisfies Chat
	return { chat, set }
}

function answer(id: string, pending = false): ChatMessage {
	return { id, role: 'assistant', content: 'done', success: true, createdAt: '', pending }
}

function conversation(id: string, extra: Partial<Conversation> = {}): Conversation {
	return { id, title: id, createdAt: '', updatedAt: '', ...extra }
}

function pool(
	options: { keepSettled?: number; isRunFinished?: (jobId: string) => Promise<boolean> } = {}
) {
	const chats: ReturnType<typeof fakeChat>[] = []
	const hosts = { resumeTurn: vi.fn(), dispose: vi.fn() }
	const created = new FlowChatPool<{ chat: Chat }>({
		createChat: () => {
			const fake = fakeChat()
			chats.push(fake)
			return fake.chat
		},
		createHost: (chat) => ({ chat }),
		disposeHost: hosts.dispose,
		hasQueued: () => false,
		resumeTurn: (host, turn) => hosts.resumeTurn(host.chat, turn),
		isRunFinished: options.isRunFinished ?? (async () => false),
		keepSettled: options.keepSettled,
		pollMs: 10
	})
	const chatOf = (id: string) => chats.find((c) => c.chat.getState().conversationId === id)!
	return { pool: created, chatOf, hosts }
}

describe('FlowChatPool', () => {
	it('runs a turn in each of two conversations and counts an answer that lands out of view', () => {
		const { pool: p, chatOf } = pool()
		p.select('a')
		chatOf('a').set({ status: 'streaming' })
		p.select('b')
		chatOf('b').set({ status: 'submitted' })
		expect(p.getState().activity).toEqual({ a: 'running', b: 'running' })

		chatOf('a').set({ status: 'streaming', messages: [answer('m1', true)] })
		chatOf('a').set({ status: 'idle', messages: [answer('m1')] })
		expect(p.getState().activity).toEqual({ b: 'running' })
		expect(p.getState().unread).toEqual({ a: 1 })

		p.select('a')
		expect(p.getState().unread).toEqual({})
		expect(chatOf('a').chat.destroy).not.toHaveBeenCalled()
		p.destroy()
	})

	it('polls a turn another page started until its run ends, and follows it when selected', async () => {
		let finished = false
		const { pool: p, chatOf, hosts } = pool({ isRunFinished: async () => finished })
		const turn = { jobId: 'job-1', userSeq: 7 }
		p.setListed([
			conversation('a', { runningTurn: turn }),
			conversation('b', { runningTurn: turn })
		])
		expect(p.getState().activity).toEqual({ a: 'running', b: 'running' })

		p.select('a')
		expect(hosts.resumeTurn).toHaveBeenCalledWith(chatOf('a').chat, turn)

		finished = true
		await vi.waitFor(() => expect(p.getState().activity).toEqual({}))
		p.destroy()
	})

	it('releases settled chats past the budget, never one still running', () => {
		const { pool: p, chatOf, hosts } = pool({ keepSettled: 1 })
		p.select('busy')
		chatOf('busy').set({ status: 'streaming' })
		p.select('old')
		p.select('recent')
		p.select('shown')
		expect(chatOf('old').chat.destroy).toHaveBeenCalled()
		expect(p.get('old')).toBeUndefined()
		expect(p.get('recent')).toBeDefined()
		expect(p.get('busy')).toBeDefined()
		expect(hosts.dispose).toHaveBeenCalledTimes(1)
		p.destroy()
	})
})

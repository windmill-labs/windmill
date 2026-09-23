import { describe, expect, it, vi } from 'vitest'
import type { Chat, ChatMessage, ChatState, Conversation } from 'windmill-chat'
import { FlowChatPool, type ListedConversation } from './flowChatPool'
import type { ConversationTurns, DraftSender } from './conversationTurns'

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
		// Like the SDK: the id and the loading flag are set before the first page is read.
		selectConversation: vi.fn(async (id: string) =>
			set({ conversationId: id, loadingMessages: true })
		),
		loadConversations: vi.fn(async () => []),
		deleteConversation: vi.fn(async () => {}),
		renameConversation: vi.fn(async () => {}),
		loadOlderMessages: vi.fn(async () => {}),
		refreshMessages: vi.fn(async () => {}),
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

type TestHost = DraftSender<never> & { chat: Chat; turns: ConversationTurns<never> }

function pool(
	options: {
		keepSettled?: number
		listRecent?: (page: number) => Promise<readonly ListedConversation[]>
	} = {}
) {
	const chats: ReturnType<typeof fakeChat>[] = []
	const dispose = vi.fn()
	const created = new FlowChatPool<TestHost, never>({
		createChat: () => {
			const fake = fakeChat()
			chats.push(fake)
			return fake.chat
		},
		createHost: (turns) => ({
			chat: turns.chat,
			turns,
			prepareSend: () => ({}),
			sendFailed: () => {}
		}),
		disposeHost: dispose,
		listRecent: options.listRecent ?? (async () => []),
		keepSettled: options.keepSettled,
		pollMs: 10
	})
	const chatOf = (id: string) => chats.find((c) => c.chat.getState().conversationId === id)!
	const fakeOf = (chat: Chat) => chats.find((c) => c.chat === chat)!
	const turnsOf = (id: string) => (created.get(id)!.host as TestHost).turns
	return { pool: created, chatOf, fakeOf, turnsOf, dispose }
}

const typed = (text: string) => ({ text, attachments: [] })

describe('FlowChatPool', () => {
	it('runs a turn in each of two conversations and counts an answer that lands out of view', () => {
		const { pool: p, chatOf } = pool()
		p.select('a')
		chatOf('a').set({ loadingMessages: false, status: 'streaming' })
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

	it('watches a turn another page started through the listing, and follows it when selected', async () => {
		let listed: ListedConversation[] = []
		let reads = 0
		const { pool: p, chatOf } = pool({
			listRecent: async () => {
				reads++
				return listed
			}
		})
		const turn = { jobId: 'job-1', userSeq: 7 }
		p.setListed(
			[conversation('a', { runningTurn: turn }), conversation('b', { runningTurn: turn })],
			p.listingStarted()
		)
		expect(p.getState().activity).toEqual({ a: 'running', b: 'running' })

		p.select('a')
		expect(chatOf('a').chat.resumeTurn).toHaveBeenCalledWith(turn)

		// Turn 1 ended and another one started in b: the row keeps running on the newer turn.
		listed = [conversation('b', { runningTurn: { jobId: 'job-2', userSeq: 9 } })]
		await vi.waitFor(() => expect(reads).toBeGreaterThan(0))
		expect(p.getState().activity).toEqual({ b: 'running' })

		listed = [conversation('b')]
		await vi.waitFor(() => expect(p.getState().activity).toEqual({}))
		p.destroy()
	})

	it('reads past the first page for a running row that has written nothing for a while', async () => {
		let running = true
		const quiet = () => conversation('z', running ? { runningTurn: { jobId: 'job-z', userSeq: 3 } } : {})
		const pagesRead: number[] = []
		const { pool: p } = pool({
			// Others were active since: the running row is on page 2.
			listRecent: async (page) => {
				pagesRead.push(page)
				return page === 1 ? [conversation('x'), conversation('y')] : page === 2 ? [quiet()] : []
			}
		})
		p.setListed([quiet()], p.listingStarted())
		// The poll must have read page 2 to still call it running: page 1 does not hold it.
		await vi.waitFor(() => expect(pagesRead).toContain(2))
		expect(p.getState().activity).toEqual({ z: 'running' })
		running = false
		await vi.waitFor(() => expect(p.getState().activity).toEqual({}))
		p.destroy()
	})

	it('brings running rows back once the listing answers again', async () => {
		let answers = false
		const turn = { jobId: 'job-1', userSeq: 7 }
		const { pool: p } = pool({
			listRecent: async () => {
				if (!answers) throw new Error('offline')
				return [conversation('a', { runningTurn: turn })]
			}
		})
		p.setListed([conversation('a', { runningTurn: turn })], p.listingStarted())
		expect(p.getState().activity).toEqual({ a: 'running' })
		// Three failed listings in a row: the row goes quiet rather than staying stuck.
		await vi.waitFor(() => expect(p.getState().activity).toEqual({}))
		answers = true
		await vi.waitFor(() => expect(p.getState().activity).toEqual({ a: 'running' }), { timeout: 3000 })
		p.destroy()
	})

	it('keeps what a later listing said over an earlier one that lands after it', () => {
		const { pool: p } = pool()
		const earlier = p.listingStarted()
		const later = p.listingStarted()
		p.setListed([conversation('a', { runningTurn: { jobId: 'job-2', userSeq: 9 } })], later)
		p.setListed([conversation('a')], earlier)
		expect(p.getState().activity).toEqual({ a: 'running' })
		p.destroy()
	})

	it('does not take a turn back from a listing requested before it ended', () => {
		const { pool: p, chatOf } = pool()
		p.select('a')
		chatOf('a').set({ status: 'streaming' })
		const since = p.listingStarted()
		chatOf('a').set({ status: 'idle', messages: [answer('m1')] })
		p.setListed([conversation('a', { runningTurn: { jobId: 'job-1', userSeq: 3 } })], since)
		expect(p.getState().activity).toEqual({})
		expect(chatOf('a').chat.resumeTurn).not.toHaveBeenCalled()
		p.destroy()
	})

	it('counts no answer of a first page that lands after the reader moved on, and rereads a held chat on return', () => {
		const { pool: p, chatOf } = pool()
		p.select('a')
		p.select('b')
		chatOf('a').set({ loadingMessages: false, messages: [answer('old-1'), answer('old-2')] })
		expect(p.getState().unread).toEqual({})

		p.select('a')
		expect(chatOf('a').chat.refreshMessages).toHaveBeenCalledTimes(1)
		p.destroy()
	})

	it('takes a chat back as the new one when its first message never ran', () => {
		const { pool: p, fakeOf } = pool()
		const draft = p.selected
		const { set } = fakeOf(draft.chat)
		// A new chat's first message names its conversation, then is withdrawn: the upload
		// failed, or Stop was pressed while it ran, so that conversation was never created.
		set({ conversationId: 'new-1' })
		expect(p.getState().selectedId).toBe('new-1')
		set({ conversationId: undefined })
		expect(p.getState().selectedId).toBeUndefined()
		expect(p.get('new-1')).toBeUndefined()
		expect(p.getState().activity).toEqual({})
		// The next message mints its own id on that same chat, and the pool follows it there.
		expect(p.selected).toBe(draft)
		set({ conversationId: 'new-2' })
		expect(p.getState().selectedId).toBe('new-2')
		expect(p.get('new-2')).toBe(draft)
		p.destroy()
	})

	it("hands what a withdrawn chat held to the new chat that took its place", async () => {
		const { pool: p, fakeOf } = pool()
		const withdrawn = p.selected
		const { set } = fakeOf(withdrawn.chat)
		set({ conversationId: 'new-1' })
		;(withdrawn.host as TestHost).turns.adopt(typed('typed while uploading'))
		// The reader opens another new chat while the first message is still uploading.
		const kept = p.newChat()
		expect(kept).not.toBe(withdrawn)
		set({ conversationId: undefined })
		// Not released yet: the send reports what it could not do after this.
		expect(withdrawn.chat.destroy).not.toHaveBeenCalled()
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(withdrawn.chat.destroy).toHaveBeenCalled()
		expect((kept.host as TestHost).turns.takeReturned().text).toBe('typed while uploading')
		expect(p.selected).toBe(kept)
		p.destroy()
	})

	it('keeps a settled chat that still holds a queued message, and marks its row', () => {
		const { pool: p, chatOf, turnsOf } = pool({ keepSettled: 1 })
		p.select('typed')
		turnsOf('typed').queue(typed('later'))
		for (const id of ['b', 'c', 'd']) p.select(id)
		expect(p.get('typed')).toBeDefined()
		expect(p.getState().queued).toEqual({ typed: true })
		expect(chatOf('typed').chat.destroy).not.toHaveBeenCalled()
		// Once it is taken back, the chat is releasable like any other.
		turnsOf('typed').takeHeld()
		p.select('e')
		expect(p.get('typed')).toBeUndefined()
		p.destroy()
	})

	it('keeps a chat whose send has not settled, so a refusal can hand its draft back', async () => {
		const { pool: p, chatOf, turnsOf } = pool({ keepSettled: 1 })
		p.select('a')
		let refuse = (_e: Error) => {}
		chatOf('a').chat.sendMessage.mockImplementationOnce(
			() => new Promise<void>((_, reject) => (refuse = reject))
		)
		const sent = turnsOf('a').send(typed('with a file'))
		for (const id of ['b', 'c', 'd']) p.select(id)
		expect(p.get('a')).toBeDefined()
		refuse(new Error('upload failed (500)'))
		await sent
		expect(turnsOf('a').takeReturned().text).toBe('with a file')
		p.destroy()
	})

	it('never releases the conversation being left, whose composer still holds its draft', () => {
		const { pool: p, fakeOf } = pool({ keepSettled: 1 })
		for (const id of ['a', 'b', 'c']) p.select(id)
		const started = p.newChat()
		fakeOf(started.chat).set({ conversationId: 'n' })
		p.select('d')
		expect(p.get('n')).toBe(started)
		expect(started.chat.destroy).not.toHaveBeenCalled()
		p.destroy()
	})

	it('releases settled chats past the budget, never one still running', () => {
		const { pool: p, chatOf, dispose } = pool({ keepSettled: 1 })
		p.select('busy')
		chatOf('busy').set({ status: 'streaming' })
		p.select('old')
		p.select('recent')
		p.select('shown')
		expect(chatOf('old').chat.destroy).toHaveBeenCalled()
		expect(p.get('old')).toBeUndefined()
		expect(p.get('recent')).toBeDefined()
		expect(p.get('busy')).toBeDefined()
		expect(dispose).toHaveBeenCalledTimes(1)
		p.destroy()
	})
})

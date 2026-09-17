import { describe, expect, it, vi } from 'vitest'
import { TurnRunningError, type Chat, type ChatMessage, type ChatState } from 'windmill-chat'
import { FlowChatViewHost, toDisplayMessages } from './flowChatViewHost.svelte'

function message(partial: Partial<ChatMessage> & Pick<ChatMessage, 'role'>): ChatMessage {
	return {
		id: partial.id ?? `${partial.role}-${Math.random()}`,
		content: '',
		success: true,
		createdAt: '2026-09-16T10:00:00Z',
		pending: false,
		...partial
	}
}

function idleState(partial: Partial<ChatState> = {}): ChatState {
	return {
		conversationId: 'c1',
		messages: [],
		status: 'idle',
		error: undefined,
		conversations: [],
		history: 'server',
		loadingMessages: false,
		hasMoreMessages: false,
		...partial
	}
}

/** A `Chat` whose state the test drives by hand. */
function fakeChat(initial: ChatState = idleState()) {
	let state = initial
	const listeners = new Set<(s: ChatState) => void>()
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
		selectConversation: vi.fn(async () => {}),
		loadConversations: vi.fn(async () => []),
		deleteConversation: vi.fn(async () => {}),
		renameConversation: vi.fn(async () => {}),
		loadOlderMessages: vi.fn(async () => {}),
		refreshMessages: vi.fn(async () => {}),
		destroy: vi.fn()
	} satisfies Chat
	const set = (patch: Partial<ChatState>) => {
		state = { ...state, ...patch }
		for (const listener of listeners) listener(state)
	}
	return { chat, set }
}

describe('toDisplayMessages', () => {
	it('maps user, assistant and tool rows, marking the user message of a failed turn', () => {
		const rows = [
			message({ role: 'user', content: 'hi' }),
			message({
				role: 'assistant',
				content: 'hello',
				reasoning: 'thinking',
				stepName: 'agent',
				jobId: 'job-1',
				createdAt: '2026-09-16T10:00:01Z'
			}),
			message({ role: 'user', content: 'again' }),
			// A tool still running has no text yet: the server words the row on its return.
			message({
				role: 'tool',
				id: 'tool-row',
				pending: true,
				tool: { name: 'search', status: 'running', arguments: '{"q":"x"}' }
			}),
			message({ role: 'assistant', content: 'boom', success: false })
		]
		const display = toDisplayMessages(rows)
		expect(display[0]).toEqual({ role: 'user', index: 0, content: 'hi', error: undefined })
		expect(display[1]).toMatchObject({
			role: 'assistant',
			content: 'hello',
			reasoning: 'thinking',
			stepName: 'agent',
			jobId: 'job-1',
			createdAt: '2026-09-16T10:00:01Z',
			streaming: undefined
		})
		expect(display[2]).toEqual({ role: 'user', index: 1, content: 'again', error: true })
		expect(display[3]).toMatchObject({
			role: 'tool',
			tool_call_id: 'tool-row',
			content: 'Running search',
			toolName: 'search',
			parameters: { q: 'x' },
			showDetails: true,
			isLoading: true,
			error: undefined
		})
	})

	it('shows a call the turn finished without as an error, not as still running', () => {
		const display = toDisplayMessages([
			message({ role: 'user', content: 'hi' }),
			message({ role: 'tool', tool: { name: 'search', status: 'running' } })
		])
		expect(display[1]).toMatchObject({
			content: 'search did not finish',
			error: 'search did not finish',
			isLoading: false
		})

		const structured = toDisplayMessages([
			message({ role: 'user', content: 'hi' }),
			message({
				role: 'tool',
				tool: { name: 'structured_output', status: 'running', arguments: '{"n":1}' }
			})
		])
		expect(structured[1]).toMatchObject({ content: 'Running structured_output', error: undefined })
	})

	it('shows the thinking that led to a call as its own card, and retries by transcript position', async () => {
		const rows = [
			message({ role: 'user', content: 'first' }),
			message({
				role: 'tool',
				content: 'Used search tool',
				reasoning: 'why',
				tool: { name: 'search', status: 'success' }
			}),
			message({ role: 'assistant', content: 'done' }),
			message({ role: 'user', content: 'second' })
		]
		const display = toDisplayMessages(rows)
		expect(display.map((m) => [m.role, m.content])).toEqual([
			['user', 'first'],
			['assistant', ''],
			['tool', 'Used search tool'],
			['assistant', 'done'],
			['user', 'second']
		])
		expect(display[1]).toMatchObject({ reasoning: 'why' })
		expect(display[1]).not.toHaveProperty('streaming')

		const { chat } = fakeChat(idleState({ messages: rows }))
		const host = new FlowChatViewHost(chat)
		host.retryRequest(4)
		await vi.waitFor(() =>
			expect(chat.sendMessage).toHaveBeenCalledWith('second', expect.anything())
		)
		host.dispose()
	})

	it('does not flag a turn whose tool failed but whose agent still answered', () => {
		const display = toDisplayMessages([
			message({ role: 'user', content: 'try' }),
			message({
				role: 'tool',
				content: 'Error executing search',
				success: false,
				tool: { name: 'search', status: 'error' }
			}),
			message({ role: 'assistant', content: 'search is down, here is what I know' })
		])
		expect(display[0]).toMatchObject({ role: 'user', error: undefined })
	})

	it('offers no retry while the turn whose tool failed is still running', () => {
		const messages = [
			message({ role: 'user', content: 'first' }),
			message({ role: 'assistant', content: 'boom', success: false }),
			message({ role: 'user', content: 'try' }),
			message({
				role: 'tool',
				content: 'Error executing search',
				success: false,
				tool: { name: 'search', status: 'error' }
			})
		]
		const running = toDisplayMessages(messages, true)
		expect(running[0]).toMatchObject({ role: 'user', error: true })
		expect(running[2]).toMatchObject({ role: 'user', error: undefined })
		expect(toDisplayMessages(messages, false)[2]).toMatchObject({ role: 'user', error: true })
	})

	it('shows what a failed tool returned as its error, not the row label', () => {
		const display = toDisplayMessages([
			message({
				role: 'tool',
				content: 'Error executing lookup_stock',
				success: false,
				tool: {
					name: 'lookup_stock',
					status: 'error',
					result: '{"message":"stock service unavailable","name":"Error","stack":"at main"}'
				}
			}),
			message({
				role: 'tool',
				content: 'Error executing mcp_search',
				success: false,
				tool: { name: 'mcp_search', status: 'error', result: 'connection refused' }
			})
		])
		expect(display[0]).toMatchObject({
			content: 'Error executing lookup_stock',
			error: 'stock service unavailable'
		})
		expect(display[1]).toMatchObject({ error: 'connection refused' })
	})

	it('flags the streaming assistant message and a failed tool', () => {
		const display = toDisplayMessages([
			message({ role: 'assistant', content: 'partial', pending: true }),
			message({
				role: 'tool',
				content: 'Error executing search',
				success: false,
				tool: { name: 'search', status: 'error' }
			})
		])
		expect(display[0]).toMatchObject({ role: 'assistant', streaming: true })
		expect(display[1]).toMatchObject({
			role: 'tool',
			error: 'Error executing search',
			showDetails: false,
			isLoading: false
		})
	})
})

describe('FlowChatViewHost', () => {
	it('sends the text with the additional inputs and reports loading from the status', async () => {
		const { chat, set } = fakeChat()
		const host = new FlowChatViewHost(chat, { additionalInputs: () => ({ tone: 'brief' }) })
		expect(host.loading).toBe(false)
		expect(await host.sendRequest({ instructions: '  hello ' })).toBe(true)
		expect(chat.sendMessage).toHaveBeenCalledWith('hello', { inputs: { tone: 'brief' } })
		expect(await host.sendRequest({ instructions: '   ' })).toBe(false)
		set({ status: 'streaming' })
		expect(host.loading).toBe(true)
		set({ status: 'idle' })
		expect(host.loading).toBe(false)
		host.dispose()
	})

	it('queues a message typed during a turn and sends it once the turn is released', async () => {
		const { chat, set } = fakeChat()
		// The chat publishes `idle` from inside `sendMessage`, before that call returns and
		// releases the turn; a flush in between is refused as a second turn.
		let releaseTurn = () => {}
		chat.sendMessage.mockImplementationOnce(
			() => new Promise<void>((resolve) => (releaseTurn = resolve))
		)
		const host = new FlowChatViewHost(chat)
		void host.sendRequest({ instructions: 'start' })
		set({ status: 'streaming' })
		host.queueMessage('first')
		host.queueMessage('second')
		expect(host.queuedMessage).toBe('first\nsecond')
		set({ status: 'idle' })
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(chat.sendMessage).toHaveBeenCalledTimes(1)
		releaseTurn()
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(host.queuedMessage).toBe('')
		expect(chat.sendMessage).toHaveBeenLastCalledWith('first\nsecond', { inputs: undefined })
		host.dispose()
	})

	it('hands the queue back when a failed flow settles as idle', () => {
		const { chat, set } = fakeChat(
			idleState({ status: 'streaming', messages: [message({ role: 'user', content: 'go' })] })
		)
		const host = new FlowChatViewHost(chat)
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		host.queueMessage('later')
		// A failed flow is answered with its error and the status still returns to idle.
		set({
			status: 'idle',
			messages: [
				message({ role: 'user', content: 'go' }),
				message({ role: 'assistant', content: 'boom', success: false })
			]
		})
		expect(prependText).toHaveBeenCalledWith('later')
		expect(chat.sendMessage).not.toHaveBeenCalled()
		host.dispose()
	})

	it('hands the queue back instead of sending while sending is disabled', async () => {
		const { chat, set } = fakeChat(idleState({ status: 'streaming' }))
		let deploying = false
		const host = new FlowChatViewHost(chat, { sendDisabled: () => deploying })
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		host.queueMessage('after deploy')
		// A deployment starts while the turn is still running; the composer is disabled.
		deploying = true
		set({ status: 'idle' })
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(chat.sendMessage).not.toHaveBeenCalled()
		expect(prependText).toHaveBeenCalledWith('after deploy')
		expect(host.queuedMessage).toBe('')
		host.dispose()
	})

	it('drops a flush still waiting on the turn once disposed', async () => {
		const { chat, set } = fakeChat()
		let releaseTurn = () => {}
		chat.sendMessage.mockImplementationOnce(
			() => new Promise<void>((resolve) => (releaseTurn = resolve))
		)
		const host = new FlowChatViewHost(chat)
		void host.sendRequest({ instructions: 'start' })
		set({ status: 'streaming' })
		host.queueMessage('never')
		set({ status: 'idle' })
		host.dispose()
		releaseTurn()
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(chat.sendMessage).toHaveBeenCalledTimes(1)
	})

	it('hands the text back when the chat refuses the turn', async () => {
		const { chat } = fakeChat()
		chat.sendMessage.mockRejectedValueOnce(new Error('a message is already being answered'))
		const host = new FlowChatViewHost(chat)
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		await host.sendRequest({ instructions: 'kept' })
		expect(prependText).toHaveBeenCalledWith('kept')
		host.dispose()
	})

	it('follows the turn a conversation is still answering and sends the refused message after it', async () => {
		const { chat, set } = fakeChat()
		const turn = { jobId: 'job-9', userSeq: 41 }
		chat.sendMessage.mockRejectedValueOnce(new TurnRunningError('still answering', turn))
		let releaseResumed = () => {}
		chat.resumeTurn.mockImplementationOnce(
			() => new Promise<void>((resolve) => (releaseResumed = resolve))
		)
		const host = new FlowChatViewHost(chat)
		await host.sendRequest({ instructions: 'after it' })
		expect(chat.resumeTurn).toHaveBeenCalledWith(turn)
		expect(host.queuedMessage).toBe('after it')
		set({ status: 'streaming' })
		set({ status: 'idle' })
		releaseResumed()
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(chat.sendMessage).toHaveBeenLastCalledWith('after it', { inputs: undefined })
		host.dispose()
	})

	it('keeps text handed back while no composer is mounted for the next one', () => {
		const { chat } = fakeChat(idleState({ status: 'streaming' }))
		const host = new FlowChatViewHost(chat)
		host.queueMessage('typed before leaving')
		host.cancel()
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		expect(prependText).toHaveBeenCalledWith('typed before leaving')
		host.dispose()
	})

	it('hands the queue back to the composer on Stop and on a failed turn', async () => {
		const { chat, set } = fakeChat(idleState({ status: 'streaming' }))
		const host = new FlowChatViewHost(chat)
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		host.queueMessage('later')
		host.cancel()
		expect(chat.stop).toHaveBeenCalled()
		expect(prependText).toHaveBeenCalledWith('later')
		expect(host.queuedMessage).toBe('')

		host.queueMessage('after error')
		set({ status: 'error' })
		expect(prependText).toHaveBeenLastCalledWith('after error')
		expect(chat.sendMessage).not.toHaveBeenCalled()
		host.dispose()
	})

	it('offers no retry on a turn the reader stopped', () => {
		const failedTool = message({
			role: 'tool',
			content: 'Error executing search',
			success: false,
			tool: { name: 'search', status: 'error' }
		})
		const { chat, set } = fakeChat(
			idleState({
				status: 'streaming',
				messages: [message({ id: 'live', role: 'user', content: 'go' }), failedTool]
			})
		)
		const host = new FlowChatViewHost(chat)
		host.cancel()
		expect(chat.stop).toHaveBeenCalled()
		set({ status: 'idle' })
		expect(host.displayMessages[0]).toMatchObject({ role: 'user', error: undefined })
		// The chat re-reads the rows: the user message gets its row id, and the cancelled
		// flow's failure lands as the answer.
		const cancelled = message({ role: 'assistant', content: 'Job canceled', success: false })
		set({
			messages: [
				message({ id: 'live', serverId: 'row-2', role: 'user', content: 'go' }),
				failedTool,
				cancelled
			]
		})
		expect(host.displayMessages[0]).toMatchObject({ role: 'user', error: undefined })
		// Reopened, with an older page that failed in front: messages are rebuilt from rows.
		set({
			messages: [
				message({ id: 'row-0', serverId: 'row-0', role: 'user', content: 'before' }),
				message({ role: 'assistant', content: 'boom', success: false }),
				message({ id: 'row-2', serverId: 'row-2', role: 'user', content: 'go' }),
				failedTool,
				cancelled
			]
		})
		expect(host.displayMessages[0]).toMatchObject({ content: 'before', error: true })
		expect(host.displayMessages[2]).toMatchObject({ content: 'go', error: undefined })
		// The next turn is not stopped and fails on its own.
		set({
			messages: [
				...chat.getState().messages,
				message({ role: 'user', content: 'again' }),
				message({ role: 'assistant', content: 'boom', success: false })
			]
		})
		expect(host.displayMessages.at(-2)).toMatchObject({ role: 'user', error: true })
		host.dispose()
	})

	it('stops following the chat once disposed', () => {
		const { chat, set } = fakeChat()
		const host = new FlowChatViewHost(chat)
		host.dispose()
		set({ status: 'streaming' })
		expect(host.loading).toBe(false)
	})
})

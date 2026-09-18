import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Chat, ChatMessage, ChatState } from 'windmill-chat'
import { FlowChatViewHost, toDisplayMessages } from './flowChatViewHost.svelte'

vi.mock('$lib/gen', () => ({
	JobService: { getJobArgs: vi.fn() }
}))
vi.mock('$lib/toast', () => ({ sendUserToast: vi.fn() }))

import { JobService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'

const getJobArgs = vi.mocked(JobService.getJobArgs)
const toast = vi.mocked(sendUserToast)

beforeEach(() => {
	getJobArgs.mockReset()
	toast.mockReset()
})

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
		stop: vi.fn(async () => {}),
		newConversation: vi.fn(),
		selectConversation: vi.fn(async () => {}),
		loadConversations: vi.fn(async () => []),
		deleteConversation: vi.fn(async () => {}),
		loadOlderMessages: vi.fn(async () => {}),
		renameConversation: vi.fn(async () => {}),
		destroy: vi.fn()
	} satisfies Chat
	return { chat, set }
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

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
		expect(display[0]).toEqual({
			role: 'user',
			index: 0,
			content: 'hi',
			error: undefined,
			images: undefined,
			contextElements: undefined
		})
		expect(display[1]).toMatchObject({
			role: 'assistant',
			content: 'hello',
			reasoning: 'thinking',
			jobId: 'job-1',
			createdAt: '2026-09-16T10:00:01Z',
			streaming: undefined
		})
		expect(display[2]).toMatchObject({ role: 'user', index: 1, content: 'again', error: true })
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

	// A row stored before the worker kept the call has only its sentence: the card names the
	// tool and links its job, and has no details to open.
	it('builds a tool card from the row alone, with its job link', () => {
		const display = toDisplayMessages([
			message({
				role: 'tool',
				content: 'Used lookup tool',
				jobId: 'tool-job',
				tool: { name: 'lookup', status: 'success', arguments: '{"id":1}', result: '"ok"' }
			}),
			message({
				role: 'tool',
				content: 'Used search_docs tool',
				jobId: 'older-job',
				tool: { name: 'search_docs', status: 'success' }
			})
		])
		expect(display[0]).toMatchObject({
			toolName: 'lookup',
			parameters: { id: 1 },
			result: 'ok',
			showDetails: true,
			jobId: 'tool-job'
		})
		expect(display[1]).toMatchObject({
			toolName: 'search_docs',
			parameters: undefined,
			result: undefined,
			showDetails: false,
			jobId: 'older-job'
		})
	})

	it('labels answers with their step only once the transcript names more than one', () => {
		const one = toDisplayMessages([
			message({ role: 'assistant', content: 'a', stepName: 'agent' }),
			message({ role: 'assistant', content: 'b', stepName: 'agent' })
		])
		expect(one.map((m) => (m.role === 'assistant' ? m.stepName : null))).toEqual([
			undefined,
			undefined
		])
		const two = toDisplayMessages([
			message({ role: 'assistant', content: 'a', stepName: 'agent' }),
			message({ role: 'assistant', content: 'b', stepName: 'reviewer' })
		])
		expect(two.map((m) => (m.role === 'assistant' ? m.stepName : null))).toEqual([
			'agent',
			'reviewer'
		])
	})

	it('shows the files a user message carried, read from the message itself', () => {
		const display = toDisplayMessages(
			[
				message({
					role: 'user',
					content: 'go',
					attachments: [
						{ input: 'user_attachments', s3: 'chat/u1/shot.png' },
						{ input: 'user_attachments', s3: 'chat/u1/notes.pdf' }
					]
				})
			],
			false,
			new Set(),
			{ workspace: 'ws' }
		)
		expect(display[0]).toMatchObject({
			role: 'user',
			images: [{ name: 'shot.png' }],
			contextElements: [{ title: 'notes.pdf' }]
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
		await flush()
		expect(chat.sendMessage).toHaveBeenCalledTimes(1)
		releaseTurn()
		await flush()
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
		await flush()
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
		await flush()
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

	describe('retry', () => {
		const failedTurn = () =>
			idleState({
				messages: [
					message({ role: 'user', content: 'go', jobId: 'flow-job' }),
					message({ role: 'assistant', content: 'boom', success: false })
				]
			})

		it("replays the turn with the inputs its run had, not the composer's", async () => {
			const { chat } = fakeChat(failedTurn())
			getJobArgs.mockResolvedValueOnce({ user_message: 'go', tone: 'terse', model: 'old' } as any)
			const host = new FlowChatViewHost(chat, {
				workspace: () => 'ws',
				additionalInputs: () => ({ tone: 'brief', model: 'new' }),
				inputsShownInComposer: () => ['model']
			})
			await host.retryRequest(0)
			expect(getJobArgs).toHaveBeenCalledWith({ workspace: 'ws', id: 'flow-job' })
			// The composer's own control wins for what it shows; everything else replays.
			expect(chat.sendMessage).toHaveBeenCalledWith('go', {
				inputs: { tone: 'terse', model: 'new' }
			})
			host.dispose()
		})

		it('falls back to a plain resend once the job is purged', async () => {
			const { chat } = fakeChat(failedTurn())
			getJobArgs.mockRejectedValueOnce(Object.assign(new Error('gone'), { status: 404 }))
			const host = new FlowChatViewHost(chat, {
				workspace: () => 'ws',
				additionalInputs: () => ({ tone: 'brief' })
			})
			await host.retryRequest(0)
			expect(chat.sendMessage).toHaveBeenCalledWith('go', { inputs: { tone: 'brief' } })
			expect(toast).not.toHaveBeenCalled()
			host.dispose()
		})

		it('does nothing but say so when the run cannot be read', async () => {
			const { chat } = fakeChat(failedTurn())
			getJobArgs.mockRejectedValueOnce(Object.assign(new Error('down'), { status: 500 }))
			const host = new FlowChatViewHost(chat, { workspace: () => 'ws' })
			await host.retryRequest(0)
			expect(chat.sendMessage).not.toHaveBeenCalled()
			expect(toast).toHaveBeenCalledWith('Could not read what that turn ran with. Try again.', true)
			host.dispose()
		})

		it('refuses once a turn started while the run was being read', async () => {
			const { chat, set } = fakeChat(failedTurn())
			let answer = (_: unknown) => {}
			getJobArgs.mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)) as any)
			const host = new FlowChatViewHost(chat, { workspace: () => 'ws' })
			const retried = host.retryRequest(0)
			set({ status: 'streaming' })
			answer({ user_message: 'go' })
			await retried
			expect(chat.sendMessage).not.toHaveBeenCalled()
			expect(toast).toHaveBeenCalledWith(
				'That chat started another turn. Retry once it finishes.',
				true
			)
			host.dispose()
		})

		// A quick turn can start and settle inside that read: the chat is idle again, but its
		// latest message is not the one this retry was clicked on.
		it('refuses once a turn ran and settled while the run was being read', async () => {
			const { chat, set } = fakeChat(failedTurn())
			let answer = (_: unknown) => {}
			getJobArgs.mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)) as any)
			const host = new FlowChatViewHost(chat, { workspace: () => 'ws' })
			const retried = host.retryRequest(0)
			set({ status: 'streaming' })
			set({ status: 'idle' })
			answer({ user_message: 'go' })
			await retried
			expect(chat.sendMessage).not.toHaveBeenCalled()
			expect(toast).toHaveBeenCalledWith(
				'That chat started another turn. Retry once it finishes.',
				true
			)
			host.dispose()
		})
	})

	describe('streaming reveal', () => {
		/** A scheduler the test steps by hand, and a clock it advances. */
		function manualReveal() {
			let time = 0
			const queue: (() => void)[] = []
			return {
				options: {
					instant: false,
					now: () => time,
					schedule: (cb: () => void) => (queue.push(cb), cb),
					cancel: (handle: unknown) => {
						const i = queue.indexOf(handle as () => void)
						if (i >= 0) queue.splice(i, 1)
					}
				},
				tick: (ms: number) => {
					time += ms
					const due = queue.splice(0)
					for (const cb of due) cb()
				}
			}
		}

		it('paces a streaming answer and shows it whole once it settles', () => {
			const { chat, set } = fakeChat(idleState({ status: 'streaming' }))
			const reveal = manualReveal()
			const host = new FlowChatViewHost(chat, { revealOptions: reveal.options })
			const streaming = message({ role: 'assistant', id: 'a1', content: '', pending: true })
			set({ messages: [{ ...streaming, content: 'The answer, in one burst of text.' }] })
			const shown = () => (host.displayMessages[0] as { content: string }).content
			// Nothing is revealed until the pacer's first frame, and that frame shows a slice.
			expect(shown()).toBe('')
			reveal.tick(16)
			expect(shown().length).toBeGreaterThan(0)
			expect(shown().length).toBeLessThan('The answer, in one burst of text.'.length)
			expect('The answer, in one burst of text.'.startsWith(shown())).toBe(true)
			// Settled: the whole text, whatever the pacer had got to.
			set({
				status: 'idle',
				messages: [{ ...streaming, content: 'The answer, in one burst of text.', pending: false }]
			})
			expect(shown()).toBe('The answer, in one burst of text.')
			host.dispose()
		})

		it('shows a paced row whole once a tool card follows it', () => {
			const { chat, set } = fakeChat(idleState({ status: 'streaming' }))
			const reveal = manualReveal()
			const host = new FlowChatViewHost(chat, { revealOptions: reveal.options })
			const answer = message({
				role: 'assistant',
				id: 'a1',
				content: 'Let me look.',
				pending: true
			})
			set({ messages: [answer] })
			expect((host.displayMessages[0] as { content: string }).content).toBe('')
			set({
				messages: [
					answer,
					message({ role: 'tool', pending: true, tool: { name: 'lookup', status: 'running' } })
				]
			})
			expect((host.displayMessages[0] as { content: string }).content).toBe('Let me look.')
			host.dispose()
		})
	})
})

import { describe, expect, it, vi } from 'vitest'
import type { Chat, ChatMessage, ChatState } from 'windmill-chat'
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
		stop: vi.fn(async () => {}),
		newConversation: vi.fn(),
		selectConversation: vi.fn(async () => {}),
		loadConversations: vi.fn(async () => []),
		deleteConversation: vi.fn(async () => {}),
		loadOlderMessages: vi.fn(async () => {}),
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
		expect(chat.sendMessage).toHaveBeenCalledWith('hello', {
			inputs: { tone: 'brief' },
			attachments: [],
			attachmentsInput: undefined
		})
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
		expect(chat.sendMessage).toHaveBeenLastCalledWith('first\nsecond', {
			inputs: undefined,
			attachments: [],
			attachmentsInput: undefined
		})
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
		expect(prependText).toHaveBeenCalledWith('later', [], [], [])
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
		expect(prependText).toHaveBeenCalledWith('after deploy', [], [], [])
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
		expect(prependText).toHaveBeenCalledWith('kept', [], [], [])
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
		expect(prependText).toHaveBeenCalledWith('later', [], [], [])
		expect(host.queuedMessage).toBe('')

		host.queueMessage('after error')
		set({ status: 'error' })
		expect(prependText).toHaveBeenLastCalledWith('after error', [], [], [])
		expect(chat.sendMessage).not.toHaveBeenCalled()
		host.dispose()
	})

	const PNG = `data:image/png;base64,${btoa('\x89PNG')}`
	const image = { name: 'shot.webp', dataUrl: PNG, mediaType: 'image/png' } as any
	const pdf = {
		name: 'contract.pdf',
		dataUrl: `data:application/pdf;base64,${btoa('%PDF')}`,
		mediaType: 'application/pdf',
		size: 4
	}
	const listInput = { name: 'files', multiple: true }

	it('takes attachments only where the flow has an input for them', () => {
		const { chat } = fakeChat()
		const none = new FlowChatViewHost(chat)
		expect(none.supportsMessageAttachments).toBe(false)
		const list = new FlowChatViewHost(chat, { attachmentsTarget: () => listInput })
		expect(list.supportsMessageAttachments).toBe(true)
		expect(list.maxMessageAttachments).toBeUndefined()
		const single = new FlowChatViewHost(chat, {
			attachmentsTarget: () => ({ name: 'file', multiple: false }),
			attachmentsUnavailable: () => 'no storage'
		})
		expect(single.maxMessageAttachments).toBe(1)
		expect(single.attachmentsUnavailableReason).toBe('no storage')
	})

	it('hands the attachments to the chat, and drops a stored value for their input', async () => {
		const { chat } = fakeChat()
		const host = new FlowChatViewHost(chat, {
			additionalInputs: () => ({ tone: 'brief', files: [{ s3: 'stale' }] }),
			attachmentsTarget: () => listInput
		})
		await host.sendRequest({ instructions: 'read', images: [image], blobs: [pdf] })
		const [, options] = chat.sendMessage.mock.calls[0] as any
		expect(options.inputs).toEqual({ tone: 'brief' })
		expect(options.attachmentsInput).toBe(listInput)
		expect(options.attachments).toEqual([
			{ name: 'shot.webp', data: PNG, mediaType: 'image/png' },
			{ name: 'contract.pdf', data: pdf.dataUrl, mediaType: 'application/pdf' }
		])
	})

	// The composer caps as files are attached, but a queue built over several turns
	// arrives as one send; a scalar input would keep the first upload and strand the rest.
	it('re-applies a single-file cap to a merged queue', async () => {
		const { chat } = fakeChat()
		const host = new FlowChatViewHost(chat, {
			attachmentsTarget: () => ({ name: 'file', multiple: false })
		})
		await host.sendRequest({ instructions: 'read', images: [image], blobs: [pdf] })
		const [, options] = chat.sendMessage.mock.calls[0] as any
		expect(options.attachments.map((a: any) => a.name)).toEqual(['shot.webp'])
	})

	it('hands the draft back with its attachments when the upload is refused or stopped', async () => {
		const { chat } = fakeChat()
		const host = new FlowChatViewHost(chat, { attachmentsTarget: () => listInput })
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		chat.sendMessage.mockRejectedValueOnce(new Error('POST upload failed (500)'))
		await host.sendRequest({ instructions: 'read', blobs: [pdf] })
		expect(prependText).toHaveBeenCalledWith('read', [], [], [pdf])
		chat.sendMessage.mockRejectedValueOnce(new DOMException('aborted', 'AbortError'))
		await host.sendRequest({ instructions: 'again', images: [image] })
		expect(prependText).toHaveBeenLastCalledWith('again', [image], [], [])
		host.dispose()
	})

	// The chat settles a withdrawn turn as `idle`, which reads like a turn that ran; a queue
	// left waiting would then go out with the failed draft merged in front of it.
	it('does not send what was queued behind an upload that failed', async () => {
		const { chat, set } = fakeChat(
			idleState({ messages: [message({ role: 'user', content: 'earlier' })] })
		)
		let refuse = (_e: Error) => {}
		chat.sendMessage.mockImplementationOnce(
			() => new Promise<void>((_, reject) => (refuse = reject))
		)
		const host = new FlowChatViewHost(chat, { attachmentsTarget: () => listInput })
		const prependText = vi.fn()
		host.setAiChatInput({ prependText } as any)
		void host.sendRequest({ instructions: 'A', blobs: [pdf] })
		set({ status: 'submitted' })
		host.queueMessage('B')
		set({ status: 'idle' })
		refuse(new Error('upload failed (500)'))
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(chat.sendMessage).toHaveBeenCalledTimes(1)
		expect(host.queuedMessage).toBe('')
		expect(prependText.mock.calls.map((c) => c[0])).toEqual(['B', 'A'])
		expect(prependText).toHaveBeenLastCalledWith('A', [], [], [pdf])
		host.dispose()
	})

	it('queues attachments with the text and sends them together', async () => {
		const { chat, set } = fakeChat(idleState({ status: 'streaming' }))
		const host = new FlowChatViewHost(chat, { attachmentsTarget: () => listInput })
		host.queueMessage('look', [image], undefined, undefined, [pdf])
		expect(host.queuedImages).toEqual([image])
		expect(host.queuedBlobs).toEqual([pdf])
		set({ status: 'idle' })
		await new Promise((resolve) => setTimeout(resolve, 0))
		const [text, options] = chat.sendMessage.mock.calls[0] as any
		expect(text).toBe('look')
		expect(options.attachments.map((a: any) => a.name)).toEqual(['shot.webp', 'contract.pdf'])
		expect(host.queuedBlobs).toEqual([])
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

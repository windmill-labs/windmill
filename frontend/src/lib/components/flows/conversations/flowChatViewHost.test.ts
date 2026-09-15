import { describe, it, expect, vi, beforeEach } from 'vitest'
import { HelpersService, JobService } from '$lib/gen'
import { FlowChatViewHost } from './flowChatViewHost.svelte'

vi.mock('$lib/gen', () => ({
	HelpersService: { fileUpload: vi.fn() },
	FlowConversationsService: { listConversationMessages: vi.fn(), deleteFlowConversation: vi.fn() },
	JobService: { getJobArgs: vi.fn() },
	FlowService: {}
}))
vi.mock('$lib/toast', () => ({ sendUserToast: vi.fn() }))
// The host imports it only for the AIAutonomyMode enum, and the real module pulls the
// editor in behind it.
vi.mock('$lib/components/copilot/chat/AIChatManager.svelte', () => ({
	AIAutonomyMode: { DEFAULT: 'default' },
	AIMode: { GLOBAL: 'global' }
}))

/** Just enough manager for the send protocol: the host only touches these. */
function stubManager(selectedConversationId: string | undefined) {
	return {
		selectedConversationId,
		inputMessage: '',
		messages: [] as unknown[],
		isLoading: false,
		isWaitingForResponse: false,
		isDispatchingTurn: false,
		currentReasoning: '',
		isReasoningActive: false,
		wrongKindReason: undefined,
		liveRowIds: new Set<string>(),
		dispatching: [] as { id: string; on: boolean }[],
		setDispatching(id: string, on: boolean) {
			this.dispatching.push({ id, on })
		},
		createConversation: vi.fn(async function (this: any) {
			this.selectedConversationId = 'made-for-the-send'
			return 'made-for-the-send'
		}),
		sendMessage: vi.fn(async () => true),
		cancelCurrentJob: vi.fn(async () => {}),
		conversationStatus: () => 'idle',
		unreadCount: () => 0,
		busy: false,
		isConversationBusy(this: any) {
			return this.busy
		}
	}
}

const host = (manager: ReturnType<typeof stubManager>) =>
	new FlowChatViewHost(manager as any, {
		workspace: () => 'ws',
		attachmentsTarget: () => ({ name: 'files', multiple: true }),
		additionalInputs: () => ({})
	})

const anAttachment = {
	name: 'shot.png',
	dataUrl: 'data:image/png;base64,iVBORw0KGgo=',
	mediaType: 'image/png'
}

/**
 * The window between the composer taking a draft and the flow job existing. Everything that
 * has to name the turn during it — its busy state, a Stop, a message typed behind it — needs
 * a conversation to name it by, and a first message has none until one is made.
 */
describe('a send whose attachments are still uploading', () => {
	beforeEach(() => {
		vi.mocked(HelpersService.fileUpload).mockReset()
	})

	it('marks the conversation it was sent from, not whichever is open when the upload lands', async () => {
		const manager = stubManager('a')
		vi.mocked(HelpersService.fileUpload).mockImplementation(async () => {
			manager.selectedConversationId = 'b'
			return { file_key: 'k' } as any
		})
		await host(manager).sendRequest({ instructions: 'hi', blobs: [anAttachment] as any })

		expect(manager.dispatching).toEqual([
			{ id: 'a', on: true },
			{ id: 'a', on: false }
		])
		expect(manager.sendMessage.mock.calls[0]?.[2]).toBe('a')
	})

	// Stop has no job to cancel yet, so it has to be honoured when the upload lands —
	// otherwise the run starts and the reader watches a message they took back execute.
	it('does not run after a Stop pressed while it uploaded', async () => {
		const manager = stubManager('a')
		const chatHost = host(manager)
		vi.mocked(HelpersService.fileUpload).mockImplementation(async () => {
			chatHost.cancel()
			return { file_key: 'k' } as any
		})

		const started = await chatHost.sendRequest({
			instructions: 'stop me',
			blobs: [anAttachment] as any
		})

		expect(started).toBe(false)
		expect(manager.sendMessage).not.toHaveBeenCalled()
	})

	/**
	 * Two files can arrive under one name. Keyed on the name alone they race to the same
	 * object, and the agent reads whichever landed last — twice — while the other is gone.
	 */
	it('gives two attachments sharing a name their own objects', async () => {
		const manager = stubManager('a')
		const keys: string[] = []
		vi.mocked(HelpersService.fileUpload).mockImplementation((async (args: any) => {
			keys.push(args.fileKey)
			return { file_key: args.fileKey }
		}) as any)

		await host(manager).sendRequest({
			instructions: 'read both',
			blobs: [
				{
					name: 'report.pdf',
					dataUrl: 'data:application/pdf;base64,AAA',
					mediaType: 'application/pdf'
				},
				{
					name: 'report.pdf',
					dataUrl: 'data:application/pdf;base64,BBB',
					mediaType: 'application/pdf'
				}
			] as any
		})

		expect(keys).toHaveLength(2)
		expect(new Set(keys).size).toBe(2)
		// The name a reader sees is still the one they attached.
		expect(keys.every((k) => k.endsWith('/report.pdf'))).toBe(true)
	})

	// The first message of a chat has no conversation until one is made. Left to
	// `sendMessage` afterwards, nothing in this window had an id to work with.
	it('creates the conversation before uploading, so a first message can be stopped too', async () => {
		const manager = stubManager(undefined)
		const chatHost = host(manager)
		vi.mocked(HelpersService.fileUpload).mockImplementation(async () => {
			chatHost.cancel()
			return { file_key: 'k' } as any
		})

		const started = await chatHost.sendRequest({
			instructions: 'first message',
			blobs: [anAttachment] as any
		})

		expect(manager.createConversation).toHaveBeenCalled()
		expect(manager.dispatching[0]?.id).toBe('made-for-the-send')
		expect(started).toBe(false)
		expect(manager.sendMessage).not.toHaveBeenCalled()
	})
})

/**
 * Where a refused draft lands. The composer holds whichever chat is on screen, so it is the
 * right home only when the turn was that chat's and nothing is queued behind it.
 */
describe('a send that did not run', () => {
	beforeEach(() => {
		vi.mocked(HelpersService.fileUpload).mockReset()
		vi.mocked(HelpersService.fileUpload).mockResolvedValue({ file_key: 'k' } as any)
	})

	it('hands the draft back to the composer when it was the open chat with nothing waiting', async () => {
		const manager = stubManager('a')
		manager.sendMessage = vi.fn(async () => false)
		const chatHost = host(manager)
		const prepended: string[] = []
		chatHost.setAiChatInput({ prependText: (text: string) => prepended.push(text) } as any)

		await chatHost.sendRequest({ instructions: 'came back', blobs: [anAttachment] as any })

		expect(prepended).toEqual(['came back'])
	})

	// The composer on screen belongs to another conversation by now, and anything queued was
	// typed later — so the draft joins its own chat's queue, in front of what followed it.
	it('queues it in front of its own chat when that chat is not the open one', async () => {
		const manager = stubManager('a')
		manager.sendMessage = vi.fn(async () => false)
		const chatHost = host(manager)
		const prepended: string[] = []
		chatHost.setAiChatInput({ prependText: (text: string) => prepended.push(text) } as any)
		chatHost.queueMessage('typed after')
		manager.selectedConversationId = 'b'

		await chatHost.sendRequest({
			instructions: 'sent first',
			blobs: [anAttachment] as any,
			conversationId: 'a'
		})

		expect(prepended).toEqual([])
		manager.selectedConversationId = 'a'
		expect(chatHost.queuedMessage).toBe('sent first\ntyped after')
	})

	// The open chat with something already queued — reachable in the editor, where a turn
	// running in another chat refuses this one. The composer would put the older draft
	// beside the newer text, and the next settled run would send them inverted.
	it('queues it in front even when it is the open chat, if something is waiting', async () => {
		const manager = stubManager('a')
		manager.sendMessage = vi.fn(async () => false)
		const chatHost = host(manager)
		const prepended: string[] = []
		chatHost.setAiChatInput({ prependText: (text: string) => prepended.push(text) } as any)
		chatHost.queueMessage('typed after')

		await chatHost.sendRequest({ instructions: 'sent first', blobs: [anAttachment] as any })

		expect(prepended).toEqual([])
		expect(chatHost.queuedMessage).toBe('sent first\ntyped after')
	})
})

/**
 * Retry runs the turn that failed, not a new one wearing its text. The row shows the
 * attachments and inputs it ran with — read back from its job — so a retry that quietly
 * used the composer's current settings would run something other than what is on screen.
 */
describe('retrying a failed turn', () => {
	const failedRow = {
		id: 'row-1',
		message_type: 'user',
		content: 'summarise this',
		job_id: 'job-that-failed'
	}

	beforeEach(() => {
		vi.mocked(JobService.getJobArgs).mockReset()
	})

	it('runs with the arguments that turn ran with, not the ones on screen now', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			attachmentsTarget: () => ({ name: 'files', multiple: true }),
			// What the composer holds now, which must not be what the retry runs with.
			additionalInputs: () => ({ tone: 'breezy', files: [] })
		})
		vi.mocked(JobService.getJobArgs).mockResolvedValue({
			user_message: 'summarise this',
			tone: 'formal',
			files: [{ s3: 'windmill_chat_uploads/abc/report.pdf' }]
		} as any)

		await chatHost.retryRequest(0)

		expect(JobService.getJobArgs).toHaveBeenCalledWith({ workspace: 'ws', id: 'job-that-failed' })
		// The original attachment and the original input; `user_message` goes as the text.
		expect(manager.sendMessage.mock.calls[0]?.[0]).toEqual({
			tone: 'formal',
			files: [{ s3: 'windmill_chat_uploads/abc/report.pdf' }]
		})
	})

	// A purged job can no longer say what it ran with, and refusing to retry would strand
	// the reader on a failed turn. The composer is the only account left, and the row shows
	// nothing either, so the two still agree.
	it('falls back to the composer when the original run is gone', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({ tone: 'breezy' })
		})
		vi.mocked(JobService.getJobArgs).mockRejectedValue({ status: 404 })

		await chatHost.retryRequest(0)

		expect(manager.sendMessage.mock.calls[0]?.[0]).toEqual({ tone: 'breezy' })
	})

	// Anything other than a purged job leaves the turn unknowable rather than gone, and
	// running the text with today's settings would be the silent substitution this guards.
	it('refuses rather than running something else when the read fails', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({ tone: 'breezy' })
		})
		vi.mocked(JobService.getJobArgs).mockRejectedValue({ status: 500 })

		await chatHost.retryRequest(0)

		expect(manager.sendMessage).not.toHaveBeenCalled()
	})

	// A turn started in that chat while the arguments were being read: the retry must say so
	// rather than let `sendRequest` refuse it by dropping its text into the composer.
	it('says so when a send got ahead of it', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({})
		})
		vi.mocked(JobService.getJobArgs).mockImplementation(async () => {
			manager.busy = true
			return { user_message: 'summarise this' } as any
		})

		await chatHost.retryRequest(0)

		expect(manager.sendMessage).not.toHaveBeenCalled()
	})

	// The flag that refuses a second Retry is deliberately not part of `loading`, so nothing
	// disables the button — this is what stops two fetches racing into two runs.
	it('refuses a second Retry while the first is still reading', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({})
		})
		let release: (v: any) => void = () => {}
		vi.mocked(JobService.getJobArgs).mockReturnValue(
			new Promise((resolve) => (release = resolve)) as any
		)

		const first = chatHost.retryRequest(0)
		await chatHost.retryRequest(0)
		release({ user_message: 'summarise this' })
		await first

		expect(JobService.getJobArgs).toHaveBeenCalledTimes(1)
		expect(manager.sendMessage).toHaveBeenCalledTimes(1)
	})

	// The reader can pick another chat while the arguments are being read back, and the
	// replay belongs to the one they clicked Retry in — the same window the upload path pins.
	it('runs in the chat it was clicked in, not the one open when the read lands', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({})
		})
		vi.mocked(JobService.getJobArgs).mockImplementation(async () => {
			manager.selectedConversationId = 'b'
			return { user_message: 'summarise this', tone: 'formal' } as any
		})

		await chatHost.retryRequest(0)

		expect(manager.sendMessage.mock.calls[0]?.[2]).toBe('a')
	})

	// The provider and model are edited beside the transcript, not on the row — and a model
	// that cannot answer is a likely reason the turn failed, so Retry must run the new one.
	it('replays the turn but with the model the composer now holds', async () => {
		const manager = stubManager('a')
		manager.messages = [failedRow] as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			inputsShownInComposer: () => ['model'],
			additionalInputs: () => ({ model: 'claude-sonnet-5', tone: 'breezy' })
		})
		vi.mocked(JobService.getJobArgs).mockResolvedValue({
			user_message: 'summarise this',
			model: 'a-model-that-failed',
			tone: 'formal'
		} as any)

		await chatHost.retryRequest(0)

		expect(manager.sendMessage.mock.calls[0]?.[0]).toEqual({
			model: 'claude-sonnet-5',
			tone: 'formal'
		})
	})
})

// The row is named with its job as soon as the run starts, so a send that kept nothing
// would swap to the job lane mid-run and render empty for the length of a round trip.
describe('the inputs a row shows for the turn just sent', () => {
	it('come from what was sent, with no fetch, once the row has its job', async () => {
		vi.mocked(JobService.getJobArgs).mockReset()
		const manager = stubManager('a')
		const row = { id: 'temp-1', message_type: 'user', content: 'bonjour', job_id: undefined }
		manager.messages = [row]
		manager.liveRowIds = new Set(['temp-1'])
		manager.sendMessage = vi.fn(async (_args: any, nameRow: any) => {
			nameRow?.('temp-1')
			return true
		}) as any
		const chatHost = new FlowChatViewHost(manager as any, {
			workspace: () => 'ws',
			additionalInputs: () => ({ tone: 'formal' })
		})

		await chatHost.sendRequest({ instructions: 'bonjour' })
		// What `#nameTurnJob` does once the flow job exists.
		row.job_id = 'job-1' as any

		expect(chatHost.displayMessages[0]?.contextElements).toEqual([
			{ type: 'attached_file', title: 'tone', content: 'formal' }
		])
		expect(JobService.getJobArgs).not.toHaveBeenCalled()
	})
})

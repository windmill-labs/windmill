import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { FlowConversationsService } from '$lib/gen'
import { createFlowChatManager } from './FlowChatManager.svelte'

vi.mock('$lib/gen', () => ({
	FlowConversationsService: {
		listConversationMessages: vi.fn(),
		deleteFlowConversation: vi.fn()
	},
	JobService: {},
	FlowService: {}
}))
vi.mock('$lib/toast', () => ({ sendUserToast: vi.fn() }))
vi.mock('$lib/stores', () => ({
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ username: 'admin' }), () => {}) },
	workspaceStore: { subscribe: (run: (v: unknown) => void) => (run('ws'), () => {}) },
	enterpriseLicense: { subscribe: (run: (v: unknown) => void) => (run(undefined), () => {}) }
}))

/** Each `streamJob` the turn opens, and the updates the next one answers with. */
const { streamCalls, streamScript } = vi.hoisted(() => ({
	streamCalls: [] as { jobId: string; streamOffset: number | undefined }[],
	streamScript: [] as unknown[][]
}))

// Only the transport is faked. `followJob` — which owns the re-attach, the offset and the
// line buffering these tests are about — is the real one.
vi.mock('windmill-chat', async (importOriginal) => {
	const actual = await importOriginal<typeof import('windmill-chat')>()
	class FakeApi {
		async *streamJob(jobId: string, options: { streamOffset?: number } = {}) {
			streamCalls.push({ jobId, streamOffset: options.streamOffset })
			for (const update of streamScript.shift() ?? []) yield update
		}
	}
	return { ...actual, WindmillChatApi: FakeApi }
})

const rows = (conversationId: string, count: number) =>
	Array.from({ length: count }, (_, i) => ({
		id: `${conversationId}-${i}`,
		content: '',
		created_at: new Date().toISOString(),
		created_seq: i,
		conversation_id: conversationId,
		message_type: i % 2 === 0 ? 'user' : 'assistant'
	}))

function managerWithRows() {
	const manager = createFlowChatManager()
	manager.operatingWorkspace = () => 'ws'
	;(manager as any).initialize(vi.fn(), 'u/admin/flow', false)
	return manager
}

/**
 * The unread badge is driven by a watermark the manager writes when the reader leaves a
 * chat. What it must never do is keep counting a chat the reader can no longer reach —
 * there would be no row left in the sidebar to clear it.
 */
describe('unread bookkeeping', () => {
	beforeEach(() => {
		vi.mocked(FlowConversationsService.listConversationMessages).mockReset()
		vi.mocked(FlowConversationsService.deleteFlowConversation).mockReset()
	})

	it('counts rows loaded for a chat the reader is not in', async () => {
		const manager = managerWithRows()
		manager.selectedConversationId = 'open'
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue(
			rows('a', 2) as any
		)
		await manager.loadConversationMessages('a')

		expect(manager.unreadCount('a')).toBe(2)
		expect(manager.totalUnread).toBe(2)
	})

	it('stops counting a deleted chat', async () => {
		const manager = managerWithRows()
		manager.selectedConversationId = 'open'
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue(
			rows('a', 2) as any
		)
		await manager.loadConversationMessages('a')
		expect(manager.totalUnread).toBe(2)

		vi.mocked(FlowConversationsService.deleteFlowConversation).mockResolvedValue(undefined as any)
		await (manager as any).deleteConversation('a')

		expect(manager.totalUnread).toBe(0)
		expect(manager.unreadCount('a')).toBe(0)
	})

	it('treats the open chat as read', async () => {
		const manager = managerWithRows()
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue(
			rows('a', 3) as any
		)
		await manager.selectConversation('a')

		expect(manager.unreadCount('a')).toBe(0)
		expect(manager.totalUnread).toBe(0)
	})

	// The message the reader just typed came back as unread behind them, because this path
	// moves the selection without going through `selectConversation`.
	it('marks the chat being left as read when a new one is started', async () => {
		const manager = managerWithRows()
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue(
			rows('a', 2) as any
		)
		await manager.selectConversation('a')
		await manager.createConversation({ clearMessages: true })

		expect(manager.unreadCount('a')).toBe(0)
	})
})

/**
 * A queued message goes out when the turn ahead of it reaches a terminal state — not
 * merely when the chat stops looking busy. The stream dropping is the case that separates
 * the two: `onerror` ends the turn's client-side state, but the flow job it was following
 * keeps running on a worker, and starting the next turn there would interleave two runs
 * over one conversation's agent memory.
 */
describe('queued turns wait for a settled run', () => {
	it('does not flush when a turn ends without settling', () => {
		const manager = managerWithRows()
		const flushed: string[] = []
		manager.onTurnSettled = (id) => flushed.push(id)

		manager.endTurn('a')

		expect(flushed).toEqual([])
	})

	it('flushes the chat whose run settled, naming it', () => {
		const manager = managerWithRows()
		const flushed: string[] = []
		manager.onTurnSettled = (id) => flushed.push(id)

		manager.endTurn('a', { settled: true })

		expect(flushed).toEqual(['a'])
	})
})

/**
 * The server ends every SSE stream on its own clock (`TIMEOUT_SSE_STREAM`, 60s by default)
 * and expects the client to re-attach. Re-entering the run-starting path instead spawned a
 * second flow job a minute, each one writing the same conversation — so this pins that a
 * timeout follows the job it already has, and resumes where the last one stopped.
 */
describe('an SSE timeout re-attaches instead of re-running', () => {
	let live: ReturnType<typeof managerWithRows> | undefined

	beforeEach(() => {
		streamCalls.length = 0
		streamScript.length = 0
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue([] as any)
		// `test-setup.ts` makes `window` be `globalThis`, which has no `location` — and the
		// api client resolves its URLs against an absolute origin.
		;(globalThis as any).location = { origin: 'http://localhost' }
	})

	afterEach(() => {
		// The turn may still be running: its 500ms poll interval and the reconnect loop would
		// otherwise keep calling into the manager after the test. Here rather than in the
		// body, which a failing assertion would skip.
		live?.cleanup()
		live = undefined
		delete (globalThis as any).location
	})

	function turnWith(script: unknown[][]) {
		streamScript.push(...script)
		const manager = (live = managerWithRows())
		const onRunFlow = vi.fn(async () => 'job-1')
		;(manager as any).initialize(onRunFlow, 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		return { manager, onRunFlow }
	}

	it('follows the same job and carries the offset forward', async () => {
		const { manager, onRunFlow } = turnWith([
			[
				{ type: 'update', stream_offset: 42, flow_stream_job_id: 'agent-step-1' },
				{ type: 'timeout' }
			],
			// The turn is deliberately left running: what it is named while a step streams is
			// the point, and completing would clear it.
			[]
		])

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		// Stop has something to cancel before any token arrives: the flow job is named as
		// soon as it is enqueued, not when the streaming step starts.
		expect(manager.currentJobId).toBe('job-1')

		await vi.waitFor(() => expect(streamCalls).toHaveLength(2))

		// No second run, and the reconnect resumes rather than replaying the answer.
		expect(onRunFlow).toHaveBeenCalledTimes(1)
		expect(streamCalls[0]).toEqual({ jobId: 'job-1', streamOffset: undefined })
		expect(streamCalls[1]).toEqual({ jobId: 'job-1', streamOffset: 42 })

		// And it is still the flow that Stop cancels once a step is streaming. Naming the
		// streaming sub-job here instead would cancel that step and leave the steps after
		// the agent running.
		expect(manager.currentJobId).toBe('job-1')
	})

	/**
	 * A chunk is not guaranteed to end on a line boundary. Split mid-JSON, the two halves
	 * were parsed separately and both discarded, losing that token from the answer.
	 */
	it('keeps a token whose chunk ended mid-line', async () => {
		const line = `${JSON.stringify({ type: 'token_delta', content: 'from-the-stream' })}\n`
		const cut = line.indexOf('from-the') + 3
		const { manager } = turnWith([
			[
				{ type: 'update', new_result_stream: line.slice(0, cut) },
				{ type: 'update', new_result_stream: line.slice(cut) }
			]
		])

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		await vi.waitFor(() =>
			expect(manager.messages.some((m) => m.content.includes('from-the-stream'))).toBe(true)
		)
	})
})

/** Why the row has to be stamped at all is on `#nameTurnJob`; this pins that it is, and
 * that it lands in the conversation the turn was sent to. */
describe('a sent message names the run it started', () => {
	let realEventSource: unknown
	let live: ReturnType<typeof managerWithRows> | undefined

	beforeEach(() => {
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue([] as any)
		realEventSource = (globalThis as any).EventSource
		;(globalThis as any).EventSource = class {
			onmessage: unknown = null
			onerror: unknown = null
			close() {}
		}
		;(globalThis as any).location = { origin: 'http://localhost' }
	})

	afterEach(() => {
		live?.cleanup()
		live = undefined
		;(globalThis as any).EventSource = realEventSource
		delete (globalThis as any).location
	})

	it('stamps the row the turn began with, so the transcript can replay it', async () => {
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-7'),
			'u/admin/flow',
			true
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'hello'

		await manager.sendMessage(undefined, undefined, 'a')

		const userRow = manager.messages.find((m) => m.message_type === 'user')
		expect(userRow?.job_id).toBe('job-7')
	})

	// A queued message flushes into the chat it was typed in, which by then need not be the
	// one on screen — the row and its job must both land there, not in the open chat.
	it('stamps the row in the conversation the turn was sent to, not the open one', async () => {
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-8'),
			'u/admin/flow',
			true
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'open-chat'
		manager.inputMessage = 'sent to the background chat'

		await manager.sendMessage(undefined, undefined, 'background-chat')

		expect(manager.messages).toEqual([])
		await manager.selectConversation('background-chat')
		const userRow = manager.messages.find((m) => m.message_type === 'user')
		expect(userRow?.job_id).toBe('job-8')
	})
})

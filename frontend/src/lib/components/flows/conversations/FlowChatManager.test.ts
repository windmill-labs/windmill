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
const { streamCalls, streamScript, jobCompleted } = vi.hoisted(() => ({
	streamCalls: [] as { jobId: string; streamOffset: number | undefined }[],
	/** Per opened stream: the updates it answers with, or 'throw' to fail the request. */
	streamScript: [] as (unknown[] | 'throw')[],
	/** What the job says when a turn that lost its stream asks whether the run is over:
	 * `true`/`false`, or 'throw' for an API that cannot be reached. */
	jobCompleted: { value: true as boolean | 'throw', gate: undefined as Promise<void> | undefined }
}))

// Only the transport is faked. `followJob` — which owns the re-attach, the offset and the
// line buffering these tests are about — is the real one.
vi.mock('windmill-chat', async (importOriginal) => {
	const actual = await importOriginal<typeof import('windmill-chat')>()
	class FakeApi {
		async *streamJob(jobId: string, options: { streamOffset?: number } = {}) {
			streamCalls.push({ jobId, streamOffset: options.streamOffset })
			const next = streamScript.shift()
			if (next === 'throw') throw new Error('stream request failed')
			for (const update of next ?? []) yield update
		}
		async getCompletedResult() {
			if (jobCompleted.gate) await jobCompleted.gate
			if (jobCompleted.value === 'throw') throw new Error('job status unavailable')
			return { completed: jobCompleted.value, success: true, result: {} }
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

describe('a chat re-pointed at another flow', () => {
	/**
	 * The route component is reused between two flows, so the chat is re-pointed rather than
	 * rebuilt. A selection carried across would open the new flow on the old flow's
	 * conversation and send into its transcript and its agent's memory.
	 */
	it('forgets the flow it was pointed at when it is re-pointed', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue(
			rows('a', 2) as any
		)
		const manager = managerWithRows()
		await manager.selectConversation('a')
		expect(manager.selectedConversationId).toBe('a')
		expect(manager.liveRowIds.size).toBe(2)

		manager.cleanup()

		expect(manager.selectedConversationId).toBeUndefined()
		expect(manager.liveRowIds.size).toBe(0)
	})

	/**
	 * Forgetting has to hold against work already in flight: a transcript fetched for the
	 * flow just left would otherwise be written into the one that replaced it.
	 */
	it('does not let a load started before the re-point write its rows back', async () => {
		let release = () => {}
		const held = new Promise<void>((resolve) => (release = resolve))
		vi.mocked(FlowConversationsService.listConversationMessages).mockImplementation((async () => {
			await held
			return rows('a', 2)
		}) as any)
		const manager = managerWithRows()
		const loading = manager.selectConversation('a')

		manager.cleanup()
		release()
		await loading

		expect(manager.liveRowIds.size).toBe(0)
		expect(manager.selectedConversationId).toBeUndefined()
	})
})

/**
 * The messages endpoint answers oldest-first with a limit, so one request only reaches the
 * start of a turn that wrote a lot of rows — an agent calling several tools a round. Its
 * answer is among the rows that would be left behind.
 */
describe('reading a turn longer than one page', () => {
	const assistantRows = (from: number, count: number) =>
		Array.from({ length: count }, (_, i) => ({
			id: `m${from + i}`,
			conversation_id: 'a',
			message_type: 'assistant',
			content: `row ${from + i}`,
			created_at: new Date().toISOString(),
			created_seq: from + i
		}))

	it('keeps reading until a page comes back short', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValueOnce(assistantRows(1, 50) as any)
			.mockResolvedValueOnce(assistantRows(51, 12) as any)
		const manager = managerWithRows()
		manager.selectedConversationId = 'a'

		await (manager as any).pollConversationMessages('a', {})

		expect(manager.messages).toHaveLength(62)
		expect(manager.messages.at(-1)?.content).toBe('row 62')
		// The second read starts where the first stopped; a cursor that did not move would
		// re-fetch the same page and still look right from the rows alone.
		const calls = vi.mocked(FlowConversationsService.listConversationMessages).mock.calls
		expect((calls[1][0] as any).afterSeq).toBe(50)
		// Nothing to resume from still reads forward from the start: with no cursor at all the
		// endpoint answers with the newest page, which would skip everything before it.
		expect((calls[0][0] as any).afterSeq).toBe(0)
	})

	/**
	 * Reading can stop before the conversation does. What was read is then not a picture of
	 * it, and applying it would both duplicate what the temp rows already show and sweep
	 * away the only record of what was never read.
	 */
	it('keeps the temp rows a capped read did not reach', async () => {
		let seq = 0
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockImplementation((async () => {
				const batch = assistantRows(seq + 1, 50)
				seq += 50
				return batch
			}) as any)
		const manager = managerWithRows()
		manager.selectedConversationId = 'a'
		const streamed = {
			id: 'temp-answer',
			conversation_id: 'a',
			message_type: 'assistant',
			content: 'the answer as it streamed',
			created_at: new Date().toISOString(),
			created_seq: 0
		}
		manager.messages = [streamed as any]

		await (manager as any).pollConversationMessages('a', { removeTempMessages: true })

		// Nothing after the last poll of a turn would finish the read, so the rows it did get
		// are dropped rather than left showing the turn's start twice beside the temp row.
		expect(manager.messages).toEqual([streamed])
	})

	it('keeps what a capped read got when a later tick can finish it', async () => {
		let seq = 0
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockImplementation((async () => {
				const batch = assistantRows(seq + 1, 50)
				seq += 50
				return batch
			}) as any)
		const manager = managerWithRows()
		manager.selectedConversationId = 'a'

		await (manager as any).pollConversationMessages('a', {})

		// 20 requests of 50 rows, and the rows are kept, so the next tick resumes from seq 1000
		// rather than reading the conversation from the start again.
		expect(manager.messages).toHaveLength(1000)
		expect(manager.messages.at(-1)?.content).toBe('row 1000')
		await (manager as any).pollConversationMessages('a', {})
		const calls = vi.mocked(FlowConversationsService.listConversationMessages).mock.calls
		expect((calls[20][0] as any).afterSeq).toBe(1000)
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
		jobCompleted.value = true
		jobCompleted.gate = undefined
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

	function turnWith(script: (unknown[] | 'throw')[]) {
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
			[{ type: 'update', stream_offset: 42 }, { type: 'timeout' }]
		])

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		// Stop has something to cancel before any token arrives: the flow job is named as
		// soon as it is enqueued, not when the streaming step starts.
		expect(manager.currentJobId).toBe('job-1')

		await vi.waitFor(() => expect(streamCalls.length).toBeGreaterThanOrEqual(2))

		// No second run, and the reconnect resumes rather than replaying the answer.
		expect(onRunFlow).toHaveBeenCalledTimes(1)
		expect(streamCalls[0]).toEqual({ jobId: 'job-1', streamOffset: undefined })
		expect(streamCalls[1]).toEqual({ jobId: 'job-1', streamOffset: 42 })
	})

	/**
	 * `followJob` absorbs the server ending a stream, but not the request to open one
	 * failing — a restarting server, a 502. Ending the turn there would free the composer
	 * while the flow runs on, and the next turn would write the same agent memory.
	 */
	it('re-opens a failed stream from its offset rather than ending the turn', async () => {
		const { manager } = turnWith([
			[{ type: 'update', stream_offset: 7 }],
			'throw',
			// Reachable again, with nothing more to say yet.
			[]
		])

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		await vi.waitFor(() => expect(streamCalls.length).toBeGreaterThanOrEqual(3), { timeout: 5000 })

		expect(streamCalls[2]).toEqual({ jobId: 'job-1', streamOffset: 7 })
		expect(manager.isConversationBusy('a')).toBe(true)
	})

	/**
	 * An API that cannot be reached has said nothing about whether the run stopped, and the
	 * run holds the conversation's agent memory. The turn stays busy rather than guessing —
	 * Stop is the reader's way out — and settles itself once the run can be read again.
	 */
	it('stays busy while the run cannot be read, and settles once it can', async () => {
		jobCompleted.value = 'throw'
		const settled: string[] = []
		const { manager } = turnWith(['throw', 'throw', 'throw', 'throw', 'throw'])
		manager.onTurnSettled = (id) => settled.push(id)

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		await vi.waitFor(() => expect(streamCalls.length).toBeGreaterThanOrEqual(5), {
			timeout: 20000
		})
		expect(manager.isConversationBusy('a')).toBe(true)
		expect(settled).toEqual([])

		jobCompleted.value = true

		await vi.waitFor(() => expect(settled).toEqual(['a']), { timeout: 20000 })
		expect(manager.isConversationBusy('a')).toBe(false)
	}, 60000)

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
	let live: ReturnType<typeof managerWithRows> | undefined

	beforeEach(() => {
		// These turns stream too, so they draw on the shared script the block above resets.
		streamCalls.length = 0
		streamScript.length = 0
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue([] as any)
		;(globalThis as any).location = { origin: 'http://localhost' }
	})

	afterEach(() => {
		live?.cleanup()
		live = undefined
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

/**
 * Nothing in the browser survives a reload. A chat whose flow is still running would read
 * as idle, and the composer would take a message that writes the same agent memory as the
 * turn already in flight.
 */
describe('a conversation opened while its run is still going', () => {
	let live: ReturnType<typeof managerWithRows> | undefined

	beforeEach(() => {
		streamCalls.length = 0
		streamScript.length = 0
		jobCompleted.value = false
		jobCompleted.gate = undefined
		;(globalThis as any).location = { origin: 'http://localhost' }
	})

	afterEach(() => {
		live?.cleanup()
		live = undefined
		delete (globalThis as any).location
	})

	/** Two turns: an older one that finished, and the newest, whose job is the one asked about. */
	function opened(jobId: string) {
		const row = (id: string, seq: number, type: string, job?: string) => ({
			id,
			conversation_id: 'a',
			message_type: type,
			content: id,
			created_at: new Date().toISOString(),
			created_seq: seq,
			job_id: job
		})
		vi.mocked(FlowConversationsService.listConversationMessages).mockResolvedValue([
			row('older-question', 0, 'user', 'job-finished-earlier'),
			row('older-answer', 1, 'assistant', 'job-finished-earlier-agent'),
			row('newest-question', 2, 'user', jobId)
		] as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'
		return manager
	}

	it('picks the turn back up from the newest turn, not an older one', async () => {
		const manager = opened('job-live')

		await manager.selectConversation('a')

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(true))
		await vi.waitFor(() => expect(streamCalls.some((call) => call.jobId === 'job-live')).toBe(true))
	})

	// The gap this closes: until the job answers, whether the chat is free is unknown, and a
	// message accepted meanwhile starts a second run against the same agent memory.
	it('holds the chat while it is asking whether a run is live', async () => {
		let release = () => {}
		jobCompleted.gate = new Promise<void>((resolve) => (release = resolve))
		jobCompleted.value = true
		const manager = opened('job-maybe')

		await manager.selectConversation('a')

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(true))
		// Stop is on offer for that whole window, and it cancels a job — so the run it would
		// be cancelling has to be named before the question, not after it.
		expect(manager.currentJobId).toBe('job-maybe')

		release()

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))
		// Nothing was taken over, so nothing is left for a later Stop to cancel.
		expect(manager.currentJobId).toBeUndefined()
	})

	/**
	 * The hold a failed load leaves behind gates the whole surface — New chat and every
	 * other conversation with it — so Stop has to be able to release it.
	 */
	it('lets Stop release a chat whose rows never loaded', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages).mockRejectedValue(
			new Error('transcript unavailable')
		)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'

		await manager.selectConversation('a')
		expect(manager.isConversationBusy('a')).toBe(true)

		await manager.cancelCurrentJob()

		expect(manager.isConversationBusy('a')).toBe(false)
	})

	it('leaves a conversation whose run is over alone', async () => {
		jobCompleted.value = true
		const manager = opened('job-done')

		await manager.selectConversation('a')
		await new Promise((resolve) => setTimeout(resolve, 300))

		expect(manager.isConversationBusy('a')).toBe(false)
		expect(streamCalls).toEqual([])
	})
})

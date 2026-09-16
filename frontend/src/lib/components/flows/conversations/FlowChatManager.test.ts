import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { FlowConversationsService, JobService } from '$lib/gen'
import { createFlowChatManager } from './FlowChatManager.svelte'

vi.mock('$lib/gen', () => ({
	FlowConversationsService: {
		listConversationMessages: vi.fn(),
		listFlowConversations: vi.fn(),
		deleteFlowConversation: vi.fn()
	},
	JobService: { cancelQueuedJob: vi.fn() },
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
	jobCompleted: {
		value: true as boolean | 'throw',
		gate: undefined as Promise<void> | undefined,
		result: {} as unknown,
		success: true
	}
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
			return {
				completed: jobCompleted.value,
				success: jobCompleted.success,
				result: jobCompleted.result
			}
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
 * The messages endpoint answers oldest-first with a limit, so one request only reaches the
 * start of a turn that wrote a lot of rows — an agent calling several tools a round. Its
 * answer is among the rows that would be left behind.
 */
/**
 * The sidebar keeps whatever its loader last answered with, and the filter starts a read
 * without waiting for the one before it. Two in flight together would leave the sidebar
 * showing whichever finished last rather than whichever was asked for last.
 */
describe('switching the conversation filter', () => {
	it('reads one filter at a time', async () => {
		const finished: string[] = []
		let release: (() => void) | undefined
		const manager = managerWithRows()
		manager.conversationKind = 'test'
		manager.conversationListComponent = {
			loadData: vi.fn(async () => {
				const kind = manager.conversationKind
				if (!release) await new Promise<void>((resolve) => (release = resolve))
				finished.push(kind)
			})
		} as any

		const first = manager.refreshConversations()
		await vi.waitFor(() => expect(release).toBeTruthy())
		manager.conversationKind = 'deployed'
		const second = manager.refreshConversations()
		release!()
		await Promise.all([first, second])

		// The second read started only once the first was done, so the list ends on the
		// filter that was asked for last.
		expect(finished).toEqual(['test', 'deployed'])
	})
})

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

	// The sidebar reads the turn's outcome off the loaded rows, and the same long turn hides
	// the message it started from, leaving a failed chat looking like an idle one.
	it('reads a failed turn that filled the page as failed', async () => {
		const manager = managerWithRows()
		manager.selectedConversationId = 'a'
		manager.messages = assistantRows(1, 50).map((row, i) =>
			i === 49 ? ({ ...row, success: false } as any) : (row as any)
		)

		expect(manager.conversationStatus('a')).toBe('error')
	})

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
	 * Reading can stop before the conversation does, and what it did read is rows of older
	 * turns rather than this one's. None of them stands for the answer on screen, so the
	 * answer stays: what a read does not account for, it does not remove.
	 */
	it('keeps the streamed answer a capped read did not reach', async () => {
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

		await (manager as any).pollConversationMessages('a', {})

		expect(manager.messages[0]).toEqual(streamed)
		// And what it did read is kept, so the next tick resumes past it rather than asking
		// for the same thousand rows again.
		expect(manager.messages).toHaveLength(1001)
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
	// The thinking indicator reads these two off the manager while the turn runs. They are
	// written by the turn and read through the open conversation, so a turn that writes them
	// somewhere the getters do not look leaves the indicator permanently off.
	it('surfaces the thinking of the turn in flight', async () => {
		const { manager } = turnWith([
			[
				{
					type: 'update',
					new_result_stream: `${JSON.stringify({
						type: 'reasoning_token_delta',
						content: 'weighing it up'
					})}\n`
				},
				{ type: 'timeout' }
			]
		])

		manager.inputMessage = 'ask something'
		await manager.sendMessage(undefined, undefined, 'a')

		await vi.waitFor(() => expect(manager.isReasoningActive).toBe(true))
		await vi.waitFor(() => expect(manager.currentReasoning).toContain('weighing it up'))
	})

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
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		vi.mocked(JobService.cancelQueuedJob)
			.mockReset()
			.mockResolvedValue('' as any)
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

	/**
	 * The whole of the run request is a window in which Stop can land, and it lands on a turn
	 * with no job to cancel. The run the request then returns is nobody's: nothing follows
	 * it, and it would hold the conversation's agent memory against the next turn.
	 */
	it('cancels a run that arrives after Stop ended the turn', async () => {
		let launch: ((jobId: string) => void) | undefined
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(() => new Promise<string>((resolve) => (launch = resolve))),
			'u/admin/flow',
			true
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'hello'

		const sent = manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(launch).toBeTruthy())
		await manager.cancelCurrentJob()
		launch!('job-late')

		expect(await sent).toBe(false)
		await vi.waitFor(() =>
			expect(vi.mocked(JobService.cancelQueuedJob)).toHaveBeenCalledWith(
				expect.objectContaining({ id: 'job-late' })
			)
		)
		// Nothing followed it either: a stream opened here would write into whatever turn
		// the chat is on by the time it answers.
		expect(streamCalls).toEqual([])
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

	/**
	 * A turn that called a lot of tools writes more rows than the page the transcript opens
	 * on, pushing the message that started it out of sight. Read from that page alone the
	 * conversation looks finished, and the composer is handed back while the run still owns
	 * the agent's memory — so the message is asked of the server by kind, which is what makes
	 * the answer independent of how many rows the turn wrote.
	 */
	it('asks the server for the message that started a turn that filled the page', async () => {
		const answerRows = Array.from({ length: 50 }, (_, i) => ({
			id: `answer-${i}`,
			conversation_id: 'a',
			message_type: i % 2 === 0 ? 'assistant' : 'tool',
			content: `round ${i}`,
			created_at: new Date().toISOString(),
			created_seq: 100 + i
		}))
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			// The newest page is this turn's answer, all of it.
			.mockResolvedValueOnce(answerRows as any)
			// The newest `user` row, which is the message that started the turn.
			.mockResolvedValueOnce([
				{
					id: 'the-question',
					conversation_id: 'a',
					message_type: 'user',
					content: 'do the thing',
					created_at: new Date().toISOString(),
					created_seq: 99,
					job_id: 'job-live'
				}
			] as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'

		await manager.selectConversation('a')

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(true))
		await vi.waitFor(() => expect(streamCalls.some((call) => call.jobId === 'job-live')).toBe(true))
		// Asked for by kind. Without that the newest row of any kind comes back, which for a
		// turn in flight is one the agent wrote — and those name the agent step's own job.
		expect(vi.mocked(FlowConversationsService.listConversationMessages)).toHaveBeenLastCalledWith(
			expect.objectContaining({ messageType: 'user', page: 1, perPage: 1 })
		)
		// The rows the turn already wrote are dropped, since the stream replays them. The
		// message that started it is not: polls only ever add what a turn wrote, so a
		// transcript emptied here would come back as answers with nothing asking them.
		expect(manager.messages.map((m) => m.id)).toEqual(['the-question'])
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
	 * Asking which turn is running is a request of its own, and it can fail on its own.
	 * Whether a run owns this conversation's memory is then precisely what is unknown, so the
	 * chat stays held rather than taking a message that would write that memory twice — and
	 * the hold gates the whole surface, New chat and every other conversation with it, so
	 * Stop has to be able to release it.
	 */
	it('keeps holding the chat when it could not ask which turn is running', async () => {
		const answerRows = Array.from({ length: 50 }, (_, i) => ({
			id: `answer-${i}`,
			conversation_id: 'a',
			message_type: 'assistant',
			content: `round ${i}`,
			created_at: new Date().toISOString(),
			created_seq: 100 + i
		}))
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValueOnce(answerRows as any)
			.mockRejectedValue(new Error('page unavailable'))
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'

		await manager.selectConversation('a')

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(true))
		// And Stop is still the way out of it.
		await manager.cancelCurrentJob()
		expect(manager.isConversationBusy('a')).toBe(false)
	})

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

	/**
	 * The row the resume names the turn with becomes the run Stop cancels, so it has to be
	 * the message that started the turn. Every other kind of row names the agent step's own
	 * job, and cancelling that leaves the steps after the agent running.
	 */
	it('holds the chat rather than resuming on a row it did not ask for', async () => {
		const answerRows = Array.from({ length: 50 }, (_, i) => ({
			id: `answer-${i}`,
			conversation_id: 'a',
			message_type: 'assistant',
			content: `round ${i}`,
			created_at: new Date().toISOString(),
			created_seq: 100 + i
		}))
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValueOnce(answerRows as any)
			// A server that ignored the filter answers with the newest row of any kind, and
			// that row carries the agent step's job.
			.mockResolvedValueOnce([
				{
					id: 'an-answer',
					conversation_id: 'a',
					message_type: 'assistant',
					content: 'the agent talking',
					created_at: new Date().toISOString(),
					created_seq: 150,
					job_id: 'job-of-the-agent-step'
				}
			] as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'

		await manager.selectConversation('a')

		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(true))
		expect(manager.currentJobId).toBeUndefined()
		expect(streamCalls).toEqual([])
	})

	/**
	 * A tool card opens on the call and closes on the result, and the result need never come
	 * — Stop is one of the ways. The worker stores no row for a call that did not finish, so
	 * nothing arriving later can close the card: ending the turn has to.
	 */
	/**
	 * A turn with no stream puts nothing on screen itself, so the rows the worker writes are
	 * the only ones it will ever have — and the worker does not wait for them. A turn that
	 * ends before any of them land would otherwise read as one that answered nothing.
	 */
	it("shows the run's own answer when no row was read back", async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		jobCompleted.value = true
		jobCompleted.success = true
		jobCompleted.result = { windmill_chat_answer: 'the answer the run produced' }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		expect(manager.messages.map((m) => m.content)).toContain('the answer the run produced')
	})

	/**
	 * A run that fails before writing anything still has to end the turn as a failure: the
	 * transcript reads a turn's outcome off its last row, and that is what offers Retry and
	 * what holds a message queued behind the turn back from running into the same failure.
	 */
	it('ends on a failure when the run failed and wrote nothing', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		jobCompleted.value = true
		jobCompleted.success = false
		jobCompleted.result = { error: { message: 'the provider refused' } }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		const last = manager.messages.at(-1)
		expect(last?.content).toBe('the provider refused')
		expect(last?.success).toBe(false)
	})

	/**
	 * The catch-up read runs for the whole of a turn with no stream, so the answer can be on
	 * screen well before the job reports itself over. Asking only what the last read brought
	 * back would call that turn empty and show the run's result beside the row it already has.
	 */
	it('does not repeat an answer the catch-up read already got', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([
				{
					id: 'db-answer',
					conversation_id: 'a',
					message_type: 'assistant',
					content: 'the answer, read back',
					created_at: new Date().toISOString(),
					created_seq: 5
				}
			] as any)
		jobCompleted.value = true
		jobCompleted.success = true
		jobCompleted.result = { windmill_chat_answer: 'the answer, read back' }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		const answers = manager.messages.filter((m) => m.content === 'the answer, read back')
		expect(answers).toHaveLength(1)
	})

	/**
	 * A queued message goes out when a turn settles. A turn that failed must not settle: the
	 * next one would run straight into the conversation that just failed, before the reader
	 * has seen why.
	 */
	it('does not arm a queued message when the run failed', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		jobCompleted.value = true
		jobCompleted.success = false
		jobCompleted.result = { error: { message: 'the provider refused' } }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'
		const settled: string[] = []
		manager.onTurnSettled = (id) => settled.push(id)

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		expect(settled).toEqual([])
	})

	/**
	 * The envelope a failure is wrapped in is a shape a flow is free to return as its answer,
	 * so the job is asked rather than the result read. A turn told it failed offers Retry and
	 * holds back whatever is queued behind it — neither belongs to a run that succeeded.
	 */
	it('asks the job rather than reading failure off the result shape', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		jobCompleted.value = true
		jobCompleted.success = true
		jobCompleted.result = { error: { message: 'not an error, just its shape' } }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'
		const settled: string[] = []
		manager.onTurnSettled = (id) => settled.push(id)

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		expect(settled).toEqual(['a'])
		expect(manager.messages.at(-1)?.success).not.toBe(false)
	})

	it('leaves no card spinning when Stop ends the turn', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([] as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.messages = [
			{
				id: 'temp-card',
				conversation_id: 'a',
				message_type: 'tool',
				content: 'Running get_time',
				created_at: new Date().toISOString(),
				created_seq: 0,
				loading: true
			}
		] as any

		await manager.cancelCurrentJob()

		expect(manager.messages[0].loading).toBe(false)
	})

	/**
	 * Stop does not reach the read that is already out, so the read fails after the composer
	 * has been handed back. Held against the chat then, it would shut a composer the reader
	 * was just given, over a run that Stop had already dealt with.
	 */
	it('stays released when the read fails after Stop', async () => {
		let fail: ((error: Error) => void) | undefined
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockImplementationOnce(() => new Promise((_, reject) => (fail = reject)) as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(vi.fn(), 'u/admin/flow', true)
		manager.operatingWorkspace = () => 'ws'

		void manager.selectConversation('a')
		await vi.waitFor(() => expect(fail).toBeTruthy())
		await manager.cancelCurrentJob()
		expect(manager.isConversationBusy('a')).toBe(false)

		fail!(new Error('transcript unavailable'))
		await vi.waitFor(() => expect(manager.isLoadingMessages).toBe(false))

		expect(manager.isConversationBusy('a')).toBe(false)
	})

	/**
	 * The read that opens a conversation is how the chat finds out whether a run owns it, so
	 * it holds the chat while it is out — and Stop can release that hold and let the reader
	 * send before the rows arrive. What comes back is then the conversation as it was before
	 * that turn: written over the live one it would drop the message just sent, and resuming
	 * from it would end the turn now streaming.
	 */
	it('leaves a live turn alone when the read it replaced answers late', async () => {
		let deliver: ((rows: unknown[]) => void) | undefined
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockImplementationOnce(() => new Promise((resolve) => (deliver = resolve)) as any)
			.mockResolvedValue([] as any)
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-new'),
			'u/admin/flow',
			true
		)
		manager.operatingWorkspace = () => 'ws'

		void manager.selectConversation('a')
		await vi.waitFor(() => expect(deliver).toBeTruthy())
		await manager.cancelCurrentJob()
		manager.inputMessage = 'ask again'
		await manager.sendMessage(undefined, undefined, 'a')

		deliver!([
			{
				id: 'before-the-send',
				conversation_id: 'a',
				message_type: 'user',
				content: 'an older question',
				created_at: new Date().toISOString(),
				created_seq: 1
			}
		])
		await vi.waitFor(() => expect(streamCalls.some((call) => call.jobId === 'job-new')).toBe(true))

		expect(manager.messages.map((m) => m.content)).toEqual(['ask again'])
		expect(manager.isConversationBusy('a')).toBe(true)
	})

	/**
	 * A turn's rows are written by tasks the run does not wait for, so a tool row can land
	 * after the answer. Reading the answer off the last row would call such a turn unanswered
	 * and show the run's result beside the answer it already has.
	 */
	it('finds the answer behind a tool row that landed after it', async () => {
		vi.mocked(FlowConversationsService.listConversationMessages)
			.mockReset()
			.mockResolvedValue([
				{
					id: 'db-answer',
					conversation_id: 'a',
					message_type: 'assistant',
					content: 'the answer',
					created_at: new Date().toISOString(),
					created_seq: 5
				},
				{
					id: 'db-tool',
					conversation_id: 'a',
					message_type: 'tool',
					content: 'Used get_time tool',
					created_at: new Date().toISOString(),
					created_seq: 6
				}
			] as any)
		jobCompleted.value = true
		jobCompleted.success = true
		jobCompleted.result = { windmill_chat_answer: 'the answer' }
		const manager = (live = managerWithRows())
		;(manager as any).initialize(
			vi.fn(async () => 'job-1'),
			'u/admin/flow',
			false
		)
		manager.operatingWorkspace = () => 'ws'
		manager.selectedConversationId = 'a'
		manager.inputMessage = 'ask'

		await manager.sendMessage(undefined, undefined, 'a')
		await vi.waitFor(() => expect(manager.isConversationBusy('a')).toBe(false))

		expect(manager.messages.filter((m) => m.content === 'the answer')).toHaveLength(1)
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

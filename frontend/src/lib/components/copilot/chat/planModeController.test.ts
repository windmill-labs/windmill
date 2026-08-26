import { beforeEach, describe, expect, it, vi } from 'vitest'

// The controller reaches `shared.ts` for `createToolDef`, which drags the editor in behind it.
vi.mock('monaco-editor', () => ({ editor: {} }))

import { PlanModeController, type PlanModeHost } from './planModeController.svelte'
import { PLAN_MODE_MESSAGES } from './planModeMessages'
import { processToolCall } from './shared'

type Doc = {
	id: string
	sessionId: string
	name: string
	content: string
	kind: 'md'
	role: 'plan'
	createdAt: number
	updatedAt: number
	version: number
	approvedVersion?: number
}

describe('PlanModeController', () => {
	let active: boolean
	let autoAccepting: boolean
	let sessionId: string
	let docs: Doc[]
	let openArtifact: ReturnType<typeof vi.fn>
	let restore: ReturnType<typeof vi.fn>
	let artifacts: any
	let controller: PlanModeController

	beforeEach(() => {
		active = true
		autoAccepting = false
		sessionId = 'session-1'
		docs = []
		openArtifact = vi.fn()
		restore = vi.fn(() => {
			active = false
		})
		artifacts = {
			// Mirrors the store: one row per session keyed by the session, created or revised in
			// a single step, and an approval that moves nothing but the pointer.
			savePlan: vi.fn(async (sessionId: string, revision: any, chatId?: string) => {
				const existing = docs.find((d) => d.sessionId === sessionId)
				if (existing) {
					if (revision.content !== existing.content) existing.version++
					existing.name = revision.name
					existing.content = revision.content
					return existing
				}
				const doc: Doc = {
					id: `plan:${sessionId}`,
					sessionId,
					name: revision.name,
					content: revision.content,
					kind: 'md',
					role: 'plan',
					createdAt: 1,
					updatedAt: 1,
					version: 1
				}
				docs = [...docs, doc]
				return doc
			}),
			approve: vi.fn(async (id: string, version: number) => {
				const doc = docs.find((d) => d.id === id)
				if (!doc) return false
				doc.approvedVersion = version
				return true
			})
		}
		const host: PlanModeHost = {
			get active() {
				return active
			},
			available: true,
			get autoAccepting() {
				return autoAccepting
			},
			isSessionChat: true,
			get sessionId() {
				return sessionId
			},
			chatId: 'chat-1',
			artifacts,
			openArtifact,
			enter: () => {
				active = true
			},
			restore
		}
		controller = new PlanModeController(host)
		controller.startRound()
	})

	const callbacks = () => ({ setToolStatus: vi.fn(), removeToolStatus: vi.fn() })
	const propose = async (summary: string, toolId = 'exit-1') => {
		const toolCallbacks = callbacks()
		controller.exitTool.onConfirmationRequested?.({ args: { summary }, toolCallbacks, toolId })
		await controller.pendingSave
		return toolCallbacks
	}

	it('persists and opens the first proposal at its current version', async () => {
		const toolCallbacks = await propose('# Add retries\n\nRetry failed work.')

		expect(docs[0]).toMatchObject({ name: 'Add retries', version: 1 })
		expect(openArtifact).toHaveBeenCalledWith('plan:session-1', 'Add retries', 'latest')
		expect(toolCallbacks.setToolStatus).toHaveBeenCalledWith(
			'exit-1',
			expect.objectContaining({ planArtifactId: 'plan:session-1', planVersion: 1 })
		)
	})

	it('revises the session plan while preserving its approval pointer', async () => {
		docs = [
			{
				id: 'plan:session-1',
				sessionId: 'session-1',
				name: 'Old',
				content: 'old',
				kind: 'md',
				role: 'plan',
				createdAt: 1,
				updatedAt: 1,
				version: 1,
				approvedVersion: 1
			}
		]

		await propose('# Revised\n\nNew approach.')

		expect(artifacts.savePlan).toHaveBeenCalledWith(
			'session-1',
			expect.not.objectContaining({ approvedVersion: expect.anything() }),
			'chat-1'
		)
		expect(docs[0]).toMatchObject({ version: 2, approvedVersion: 1 })
	})

	it('restores posture only after proposal and approval pointer are durable', async () => {
		await propose('# Durable\n\nBuild it.')
		const result = await controller.exitTool.fn({
			args: { summary: '# Durable\n\nBuild it.' },
			workspace: 'w',
			helpers: {},
			toolCallbacks: callbacks(),
			toolId: 'exit-1'
		})

		expect(docs[0].approvedVersion).toBe(1)
		expect(restore).toHaveBeenCalledOnce()
		expect(result).toBe(PLAN_MODE_MESSAGES.approvedWithDoc)
	})

	it('keeps plan mode active when persistence fails', async () => {
		artifacts.savePlan.mockRejectedValueOnce(new Error('quota'))
		const toolCallbacks = await propose('# Fails\n\nRetry later.')
		const result = await controller.exitTool.fn({
			args: { summary: '# Fails\n\nRetry later.' },
			workspace: 'w',
			helpers: {},
			toolCallbacks,
			toolId: 'exit-1'
		})

		expect(active).toBe(true)
		expect(restore).not.toHaveBeenCalled()
		expect(result).toBe(PLAN_MODE_MESSAGES.persistenceFailed)
	})

	it('leaves a card that outlived its failed save something to resolve it', async () => {
		// The write settles while the card is still waiting to be confirmed. Reporting the
		// failure onto it there would strip the confirmation the tool call is blocked on, and
		// nothing else ever resolves that — the turn would hang with no control left to click.
		artifacts.savePlan.mockRejectedValueOnce(new Error('quota'))
		// The card is the only thing that resolves the confirmation, and it offers the choice
		// for exactly as long as its status asks for one — so the click can only land while it
		// is still asking. That coupling is what makes clearing the card mid-wait fatal.
		let asking = false
		const turn = processToolCall({
			tools: [controller.exitTool],
			toolCall: {
				id: 'exit-1',
				type: 'function',
				function: { name: 'exit_plan_mode', arguments: JSON.stringify({ summary: '# P\n\nGo.' }) }
			} as any,
			helpers: {},
			workspace: 'w',
			toolCallbacks: {
				setToolStatus: (_id: string, status: any) => {
					if ('needsConfirmation' in status) asking = status.needsConfirmation
				},
				removeToolStatus: vi.fn(),
				requestConfirmation: () =>
					new Promise<boolean>((resolve) => {
						const click = () => (asking ? resolve(true) : setTimeout(click, 5))
						setTimeout(click, 5)
					}),
				isPlanModeActive: () => active,
				shouldAutoAcceptToolConfirmations: () => false
			} as any
		})

		const settled = await Promise.race([
			turn,
			new Promise((resolve) => setTimeout(() => resolve('HUNG'), 200))
		])
		expect(settled).toMatchObject({ content: PLAN_MODE_MESSAGES.persistenceFailed })
		expect(active).toBe(true)
	})

	it('refuses a second hand-over from the batch that already ended plan mode', async () => {
		// One response can carry two exit_plan_mode calls, and the tool list they run against is
		// snapshotted before the first one restores the posture. Under YOLO nothing asks, so the
		// stale call would write its own summary and stamp the user's approval on a plan no card
		// ever showed them.
		const frozenTools = [controller.exitTool]
		const handOver = (summary: string, id: string) =>
			processToolCall({
				tools: frozenTools,
				toolCall: {
					id,
					type: 'function',
					function: { name: 'exit_plan_mode', arguments: JSON.stringify({ summary }) }
				} as any,
				helpers: {},
				workspace: 'w',
				toolCallbacks: {
					setToolStatus: vi.fn(),
					removeToolStatus: vi.fn(),
					requestConfirmation: () => Promise.resolve(true),
					isPlanModeActive: () => active,
					// YOLO: every confirmation is answered for the user.
					shouldAutoAcceptToolConfirmations: () => true
				} as any
			})

		expect(await handOver('# Agreed\n\nGo.', 'exit-1')).toMatchObject({
			content: PLAN_MODE_MESSAGES.approvedWithDoc
		})
		expect(active).toBe(false)

		expect(await handOver('# Something else\n\nGo.', 'exit-2')).toMatchObject({
			content: PLAN_MODE_MESSAGES.ended
		})
		expect(docs[0]).toMatchObject({ content: '# Agreed\n\nGo.', version: 1, approvedVersion: 1 })
	})

	it('files a plan under the session that proposed it, not one swapped in mid-save', async () => {
		let released: (() => void) | undefined
		artifacts.savePlan.mockImplementationOnce(async (sessionId: string, revision: any) => {
			await new Promise<void>((resolve) => (released = resolve))
			docs = [{ id: `plan:${sessionId}`, sessionId, name: revision.name, version: 1 } as Doc]
			return docs[0]
		})
		const toolCallbacks = callbacks()
		controller.exitTool.onConfirmationRequested?.({
			args: { summary: '# Mine\n\nGo.' },
			toolCallbacks,
			toolId: 'exit-1'
		})
		await vi.waitFor(() => expect(released).toBeDefined())
		sessionId = 'session-2'
		released?.()
		await controller.pendingSave

		// The write is the proposing session's either way, but the other session must not have
		// it appear in its preview or on a card linking a document it cannot show.
		expect(docs[0].sessionId).toBe('session-1')
		expect(openArtifact).not.toHaveBeenCalled()
		expect(toolCallbacks.setToolStatus).not.toHaveBeenCalled()
	})

	it('registers only the transition available in the current posture', () => {
		expect(controller.tools.map((tool) => tool.def.function.name)).toEqual(['exit_plan_mode'])
		active = false
		expect(controller.tools.map((tool) => tool.def.function.name)).toEqual(['enter_plan_mode'])
		autoAccepting = true
		expect(controller.tools).toEqual([])
	})

	it('an old approval cannot end a newly entered round', async () => {
		await propose('# Old\n\nOld round.')
		const approval = controller.exitTool.fn({
			args: { summary: '# Old\n\nOld round.' },
			workspace: 'w',
			helpers: {},
			toolCallbacks: callbacks(),
			toolId: 'exit-1'
		})
		controller.startRound()
		await approval
		expect(active).toBe(true)
		expect(restore).not.toHaveBeenCalled()
	})
})

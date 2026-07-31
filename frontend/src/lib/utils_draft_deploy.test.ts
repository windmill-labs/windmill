import { describe, it, expect, vi, beforeEach } from 'vitest'
import { draftBaseIsStale, deployDraft } from './utils_draft_deploy'

vi.mock('$lib/gen', () => ({
	ScriptService: { getScriptByPath: vi.fn(), createScript: vi.fn() },
	FlowService: { getFlowByPath: vi.fn(), createFlow: vi.fn(), updateFlow: vi.fn() },
	DraftService: { deleteDraft: vi.fn() },
	AppService: {},
	VariableService: {},
	ResourceService: {},
	ScheduleService: {},
	HttpTriggerService: {},
	WebsocketTriggerService: {},
	PostgresTriggerService: {},
	KafkaTriggerService: {},
	NatsTriggerService: {},
	MqttTriggerService: {},
	AmqpTriggerService: {},
	SqsTriggerService: {},
	GcpTriggerService: {},
	AzureTriggerService: {},
	EmailTriggerService: {}
}))
vi.mock('$lib/userDraftDbSyncer.svelte', () => ({ UserDraftDbSyncer: { save: vi.fn() } }))
vi.mock('$lib/workspaceDrafts.svelte', () => ({ invalidateWorkspaceDrafts: vi.fn() }))
vi.mock('$lib/workspaceComparison', () => ({ invalidateWorkspaceComparison: vi.fn() }))
vi.mock('$lib/localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))
vi.mock('$lib/rawAppDeploy', () => ({ deployRawAppDraft: vi.fn() }))
vi.mock('$lib/components/raw_apps/utils', () => ({ canonicalRawAppDiffValue: vi.fn() }))
vi.mock('$lib/appDiffSides', () => ({ classicAppDraftParts: vi.fn() }))
vi.mock('$lib/utils_deployable', () => ({ TRIGGER_RUNTIME_IGNORE: [] }))

import { ScriptService, FlowService } from '$lib/gen'

// draftBaseIsStale compares a draft's base pointer against the deployed head
// of the item it was fetched with (`get_draft=true`). Shared by CompareDrafts
// and the session Edits drawer — a regression here silently hides (or
// fabricates) the "started from an older deployed version" warning.

describe('draftBaseIsStale', () => {
	it('script: stale iff the draft parent_hash differs from the deployed hash', () => {
		expect(draftBaseIsStale('script', { hash: 'v2', draft: { parent_hash: 'v1' } })).toBe(true)
		expect(draftBaseIsStale('script', { hash: 'v2', draft: { parent_hash: 'v2' } })).toBe(false)
	})

	it('script: no base pointer or no head → not stale (nothing to compare)', () => {
		expect(draftBaseIsStale('script', { hash: 'v2', draft: {} })).toBe(false)
		expect(draftBaseIsStale('script', { draft: { parent_hash: 'v1' } })).toBe(false)
	})

	it('flow: compares the pinned version_id against the deployed head', () => {
		expect(draftBaseIsStale('flow', { version_id: 7, draft: { version_id: 5 } })).toBe(true)
		expect(draftBaseIsStale('flow', { version_id: 7, draft: { version_id: 7 } })).toBe(false)
		expect(draftBaseIsStale('flow', { version_id: 7, draft: {} })).toBe(false)
	})

	it('app/raw_app: compares parent_version against the last of versions', () => {
		expect(draftBaseIsStale('app', { versions: [1, 2, 3], draft: { parent_version: 2 } })).toBe(
			true
		)
		expect(draftBaseIsStale('raw_app', { versions: [1, 2, 3], draft: { parent_version: 3 } })).toBe(
			false
		)
		expect(draftBaseIsStale('app', { versions: [], draft: { parent_version: 2 } })).toBe(false)
	})

	it('no draft on the response → not stale', () => {
		expect(draftBaseIsStale('script', { hash: 'v2' })).toBe(false)
		expect(draftBaseIsStale('script', undefined)).toBe(false)
	})
})

// Without preserve_on_behalf_of the backend rewrites on_behalf_of_email to the
// deploying user, so deploying a draft silently re-points the runnable's
// identity.

describe('deployDraft preserves on_behalf_of', () => {
	beforeEach(() => vi.clearAllMocks())

	it('script: forwards the flag when the draft carries an on_behalf_of_email', async () => {
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			hash: 'v1',
			draft: { path: 'f/admin/send_email', on_behalf_of_email: 'alice@windmill.dev' }
		} as any)

		expect(await deployDraft('script', 'f/admin/send_email', 'ws')).toEqual({ success: true })
		expect(ScriptService.createScript).toHaveBeenCalledWith(
			expect.objectContaining({
				requestBody: expect.objectContaining({
					on_behalf_of_email: 'alice@windmill.dev',
					preserve_on_behalf_of: true
				})
			})
		)
	})

	it('script: omits the flag when the draft has no on_behalf_of_email', async () => {
		vi.mocked(ScriptService.getScriptByPath).mockResolvedValueOnce({
			hash: 'v1',
			draft: { path: 'f/admin/send_email' }
		} as any)

		await deployDraft('script', 'f/admin/send_email', 'ws')
		expect(ScriptService.createScript).toHaveBeenCalledWith(
			expect.objectContaining({
				requestBody: expect.objectContaining({ preserve_on_behalf_of: undefined })
			})
		)
	})

	it('flow: forwards the flag when the draft carries an on_behalf_of_email', async () => {
		vi.mocked(FlowService.getFlowByPath).mockResolvedValueOnce({
			draft: { path: 'f/admin/notify', value: {}, on_behalf_of_email: 'alice@windmill.dev' }
		} as any)

		expect(await deployDraft('flow', 'f/admin/notify', 'ws')).toEqual({ success: true })
		expect(FlowService.updateFlow).toHaveBeenCalledWith(
			expect.objectContaining({
				requestBody: expect.objectContaining({
					on_behalf_of_email: 'alice@windmill.dev',
					preserve_on_behalf_of: true
				})
			})
		)
	})
})

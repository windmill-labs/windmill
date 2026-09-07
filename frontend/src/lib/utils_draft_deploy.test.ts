import { describe, it, expect, vi, beforeEach } from 'vitest'
import { draftBaseIsStale, deployDraft } from './utils_draft_deploy'

vi.mock('$lib/gen', () => ({
	ScriptService: { getScriptByPath: vi.fn(), createScript: vi.fn() },
	FlowService: { getFlowByPath: vi.fn(), createFlow: vi.fn(), updateFlow: vi.fn() },
	DraftService: { deleteDraft: vi.fn() },
	AppService: {},
	VariableService: {},
	ResourceService: { getResource: vi.fn(), updateResource: vi.fn(), createResource: vi.fn() },
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
vi.mock('$lib/userDraftDbSyncer.svelte', () => ({
	UserDraftDbSyncer: { save: vi.fn(), recordRemoteSync: vi.fn() }
}))
vi.mock('$lib/workspaceDrafts.svelte', () => ({ invalidateWorkspaceDrafts: vi.fn() }))
vi.mock('$lib/workspaceComparison', () => ({ invalidateWorkspaceComparison: vi.fn() }))
vi.mock('$lib/localDraftHints.svelte', () => ({ setLocalDraftHint: vi.fn() }))
vi.mock('$lib/rawAppDeploy', () => ({ deployRawAppDraft: vi.fn() }))
vi.mock('$lib/components/raw_apps/utils', () => ({ canonicalRawAppDiffValue: vi.fn() }))
vi.mock('$lib/appDiffSides', () => ({ classicAppDraftParts: vi.fn() }))
vi.mock('$lib/utils_deployable', () => ({ TRIGGER_RUNTIME_IGNORE: [] }))

import { ScriptService, FlowService, ResourceService } from '$lib/gen'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

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

// The resource branch reads the item again when the deploy lands, and falls back to the deployed
// row when the draft has gone. That row keeps its value under `value` and carries no `args` at
// all, so reading it as a draft (`value: d.args ?? {}`) would replace a live resource with `{}`.
describe('deployDraft: resource with no draft', () => {
	beforeEach(() => vi.clearAllMocks())

	it('writes nothing rather than `{}` over the deployed value', async () => {
		vi.mocked(ResourceService.getResource).mockResolvedValueOnce({
			path: 'f/support/triage_agent',
			resource_type: 'ai_agent',
			value: { system_prompt: 'deployed' }
		} as any)

		// `noop` is what lets a caller deploying one specific draft tell "nothing to promote" apart
		// from "deployed", instead of reporting an agent as deployed that was never written.
		expect(await deployDraft('resource', 'f/support/triage_agent', 'ws')).toEqual({
			success: true,
			noop: true
		})
		expect(ResourceService.updateResource).not.toHaveBeenCalled()
		expect(ResourceService.createResource).not.toHaveBeenCalled()
	})

	it('still deploys normally when the draft is there, and keys the cleanup to the row it read', async () => {
		vi.mocked(ResourceService.getResource).mockResolvedValueOnce({
			path: 'f/support/triage_agent',
			resource_type: 'ai_agent',
			value: { system_prompt: 'deployed' },
			draft_saved_at: '2026-01-01T00:00:00Z',
			draft: { path: 'f/support/triage_agent', args: { system_prompt: 'drafted' } }
		} as any)

		expect(await deployDraft('resource', 'f/support/triage_agent', 'ws')).toEqual({ success: true })
		expect(ResourceService.updateResource).toHaveBeenCalledWith(
			expect.objectContaining({
				requestBody: expect.objectContaining({ value: { system_prompt: 'drafted' } })
			})
		)
		// The draft delete that follows is conditional on this baseline. With no baseline the backend
		// deletes unconditionally, destroying a draft saved between the read and the delete without
		// ever having deployed it, so the timestamp has to be the one from the row just promoted.
		expect(UserDraftDbSyncer.recordRemoteSync).toHaveBeenCalledWith(
			{ workspace: 'ws', itemKind: 'resource', path: 'f/support/triage_agent' },
			'2026-01-01T00:00:00Z'
		)
	})
})

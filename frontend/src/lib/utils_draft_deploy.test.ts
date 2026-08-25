import { describe, it, expect, vi, beforeEach } from 'vitest'
import { draftBaseIsStale, deployDraft } from './utils_draft_deploy'

vi.mock('$lib/gen', () => ({
	ScriptService: { getScriptByPath: vi.fn(), createScript: vi.fn() },
	FlowService: { getFlowByPath: vi.fn(), createFlow: vi.fn(), updateFlow: vi.fn() },
	DraftService: { deleteDraft: vi.fn() },
	AppService: {},
	VariableService: { getVariable: vi.fn(), createVariable: vi.fn(), updateVariable: vi.fn() },
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

import { ScriptService, FlowService, VariableService } from '$lib/gen'

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

// The variable deploy bodies are built field-by-field, so a field left out of them is
// dropped with no error. `value_expires_at` is additionally tri-state on update — absent
// leaves the stored date alone, null clears it — which is why it is diffed against the
// deployed row rather than sent as-is.
describe('deployDraft variable value_expires_at', () => {
	beforeEach(() => vi.clearAllMocks())

	const DATE = '2027-03-15T08:30:00.000Z'

	function mockVariable(deployedExpiry: string | undefined, draftExpiry: string | undefined) {
		vi.mocked(VariableService.getVariable).mockResolvedValueOnce({
			path: 'u/admin/key',
			value: 'v',
			is_secret: false,
			description: '',
			value_expires_at: deployedExpiry,
			draft: {
				path: 'u/admin/key',
				variable: { value: 'v', is_secret: false, description: '' },
				value_expires_at: draftExpiry
			}
		} as any)
	}

	const updateBody = () =>
		vi.mocked(VariableService.updateVariable).mock.calls[0][0].requestBody as any

	it('sends a date the draft set on a variable that had none', async () => {
		mockVariable(undefined, DATE)
		await deployDraft('variable', 'u/admin/key', 'ws')
		expect(updateBody().value_expires_at).toBe(DATE)
	})

	it('sends null when the draft cleared the deployed date', async () => {
		mockVariable(DATE, undefined)
		await deployDraft('variable', 'u/admin/key', 'ws')
		expect(updateBody().value_expires_at).toBeNull()
	})

	it('omits it when the draft leaves the deployed date alone', async () => {
		mockVariable(DATE, DATE)
		await deployDraft('variable', 'u/admin/key', 'ws')
		expect(updateBody().value_expires_at).toBeUndefined()
	})

	it('carries the date onto a draft-only variable', async () => {
		mockVariable(undefined, DATE)
		await deployDraft('variable', 'u/admin/key', 'ws', { draftOnly: true })
		const body = vi.mocked(VariableService.createVariable).mock.calls[0][0].requestBody as any
		expect(body.value_expires_at).toBe(DATE)
	})
})

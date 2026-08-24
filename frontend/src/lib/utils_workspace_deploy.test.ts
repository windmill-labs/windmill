import { describe, it, expect, vi } from 'vitest'
import type { ProtectionRuleset, ProtectionRuleKind, User } from './gen'
import type { DeployPermission } from './utils_workspace_deploy'
import {
	checkDeployPermission,
	deployPermissionForKind,
	deployPermissionForKinds,
	kindGatedByDeployRules,
	checkPathWritePermission,
	diffActionableInDirection,
	diffCreatesInTarget,
	diffRemovesInTarget
} from './utils_workspace_deploy'

// Only the rules fetch is stubbed: the bypass logic it feeds is the code under test here,
// so it has to stay real.
let rulesets: ProtectionRuleset[] = []
vi.mock('$lib/workspaceProtectionRules.svelte', async (importOriginal) => ({
	...((await importOriginal()) as object),
	fetchProtectionRulesForWorkspace: async () => rulesets
}))

/** The row shape the fork comparison returns for an item the parent has and the
 * fork does not: one write on the fork side, whatever that write was. */
const parentOnly = { ahead: 1, behind: 0, exists_in_source: true, exists_in_fork: false }
const forkDeleted = {
	...parentOnly,
	fork_last_event_kind: 'delete',
	fork_last_event_origin: 'authored'
} as const
const forkRenamedAway = {
	...parentOnly,
	fork_last_event_kind: 'rename_from',
	fork_last_event_origin: 'authored'
} as const
const syncReverted = {
	...parentOnly,
	fork_last_event_kind: 'delete',
	fork_last_event_origin: 'sync'
} as const
const forkOnly = { ahead: 1, behind: 0, exists_in_source: false, exists_in_fork: true }
const bothSides = { ahead: 1, behind: 1, exists_in_source: true, exists_in_fork: true }

describe('deploy direction of a one-sided diff row', () => {
	it('offers a parent-only item to the fork even with no behind count', () => {
		expect(diffActionableInDirection(parentOnly, false)).toBe(true)
		expect(diffCreatesInTarget(parentOnly, false)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, false)).toBe(false)
	})

	it('keeps a parent-only row with no recorded event out of a merge into the parent', () => {
		expect(diffActionableInDirection(parentOnly, true)).toBe(false)
		// An arbitrary target has no tally behind it and does propagate the removal.
		expect(diffActionableInDirection(parentOnly, true, true)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, true)).toBe(true)
	})

	it('merges a removal the fork can show it made, and never one a sync made', () => {
		expect(diffActionableInDirection(forkDeleted, true)).toBe(true)
		expect(diffActionableInDirection(forkRenamedAway, true)).toBe(true)
		expect(diffActionableInDirection(syncReverted, true)).toBe(false)
		// Still a removal, so still opt-in rather than bulk-selected.
		expect(diffRemovesInTarget(forkDeleted, true)).toBe(true)
		// The update direction keeps offering it back whatever the fork recorded.
		expect(diffActionableInDirection(syncReverted, false)).toBe(true)
	})

	it('surfaces a fork deletion the parent also edited in both directions', () => {
		const conflict = { ...forkDeleted, behind: 1 }
		expect(diffActionableInDirection(conflict, true)).toBe(true)
		expect(diffActionableInDirection(conflict, false)).toBe(true)
	})

	it('does not resurrect a fork-only item into an update of the fork', () => {
		expect(diffActionableInDirection(forkOnly, false)).toBe(false)
		expect(diffCreatesInTarget(forkOnly, true)).toBe(true)
	})

	it('keeps a two-sided row on its counters', () => {
		expect(diffActionableInDirection(bothSides, true)).toBe(true)
		expect(diffActionableInDirection({ ...bothSides, behind: 0 }, false)).toBe(false)
		expect(diffCreatesInTarget(bothSides, true)).toBe(false)
		expect(diffRemovesInTarget(bothSides, false)).toBe(false)
	})
})

describe('per-item write permission in the deploy target', () => {
	const member = { is_admin: false, is_super_admin: false, username: 'alice', folders: ['shared'] }
	const never = async () => {
		throw new Error('folder probe should not run')
	}

	it('lets a workspace admin write anywhere', async () => {
		const admin = { is_admin: true, is_super_admin: false, username: 'root', folders: [] }
		expect(await checkPathWritePermission('dev', 'u/someone/x', admin, never)).toEqual({ ok: true })
		expect(await checkPathWritePermission('dev', 'f/locked/x', admin, never)).toEqual({ ok: true })
	})

	it('allows a user their own path and refuses someone else’s', async () => {
		expect(await checkPathWritePermission('dev', 'u/alice/x', member, never)).toEqual({ ok: true })
		const refused = await checkPathWritePermission('dev', 'u/bob/x', member, never)
		expect(refused.ok).toBe(false)
		expect(refused.reason).toContain('u/bob')
	})

	// The server's `is_owner` reads the merged `is_admin || super_admin`, so a superadmin who is a
	// plain member owns every path — refusing them here would block a write the server accepts.
	it('lets a superadmin who is a plain member write anywhere', async () => {
		const su = { is_admin: false, is_super_admin: true, username: 'root', folders: [] }
		expect(await checkPathWritePermission('dev', 'f/locked/x', su, never)).toEqual({ ok: true })
		expect(await checkPathWritePermission('dev', 'u/someone/x', su, never)).toEqual({ ok: true })
	})

	it('allows a folder in the write set without probing for it', async () => {
		expect(await checkPathWritePermission('dev', 'f/shared/x', member, never)).toEqual({ ok: true })
	})

	it('refuses a folder that exists in the target but is not writable', async () => {
		const refused = await checkPathWritePermission('dev', 'f/locked/x', member, async () => true)
		expect(refused.ok).toBe(false)
		expect(refused.reason).toContain('locked')
	})

	// The two fail-open paths. Turning either into a refusal would block a deploy the server
	// would have accepted, so they are asserted rather than left to the `catch` reading as dead.
	it('allows a folder the target does not have yet, since the deploy creates it', async () => {
		expect(
			await checkPathWritePermission('dev', 'f/brand_new/x', member, async () => false)
		).toEqual({ ok: true })
	})

	it('allows when the folder probe itself fails', async () => {
		const probeFailed = async () => {
			throw new Error('network')
		}
		expect(await checkPathWritePermission('dev', 'f/locked/x', member, probeFailed)).toEqual({
			ok: true
		})
	})
})

describe('workspace-level deploy permission', () => {
	const ruleset = (name: string, rules: ProtectionRuleKind[]): ProtectionRuleset => ({
		name,
		rules,
		bypass_users: [],
		bypass_groups: []
	})
	const member: User = {
		email: 'alice@windmill.dev',
		username: 'alice',
		is_admin: false,
		is_super_admin: false,
		operator: false,
		disabled: false,
		created_at: '2024-01-01T00:00:00Z',
		groups: [],
		folders: [],
		folders_read: [],
		folders_owners: []
	}

	const permission = (me: User, rules: ProtectionRuleset[]) => {
		rulesets = rules
		return checkDeployPermission('prod', me)
	}

	it('refuses a member when direct deployment is disabled', async () => {
		const res = await permission(member, [ruleset('lock', ['DisableDirectDeployment'])])
		expect(res.ok).toBe(false)
		expect(res.reason).toContain('prod')
	})

	// The reserved dev-workspace lock sets DisableWorkspaceForking alongside, so the advice would
	// otherwise send the user at a second blocked action.
	it('drops the fork advice when forking is blocked too', async () => {
		const res = await permission(member, [
			ruleset('lock', ['DisableDirectDeployment', 'DisableWorkspaceForking'])
		])
		expect(res.reason).not.toContain('fork')
		expect(res.reason).toContain('locally')
	})

	// wm_deployers is an implicit pass on RestrictDeployToDeployers only. Letting it
	// short-circuit the whole check — as an "is this user a deployer?" early return would —
	// walks a deployer straight through a deploy-locked workspace.
	it('still refuses a wm_deployers member when direct deployment is disabled', async () => {
		const deployer = { ...member, groups: ['wm_deployers'] }
		expect((await permission(deployer, [ruleset('lock', ['DisableDirectDeployment'])])).ok).toBe(
			false
		)
		expect((await permission(deployer, [ruleset('gate', ['RestrictDeployToDeployers'])])).ok).toBe(
			true
		)
	})

	// whoami reports is_admin and is_super_admin separately; the server sees them merged.
	it('lets a superadmin who is a plain member deploy', async () => {
		const su = { ...member, is_super_admin: true }
		expect((await permission(su, [ruleset('lock', ['DisableDirectDeployment'])])).ok).toBe(true)
	})

	it('reports the direct-deployment refusal when both rules block', async () => {
		const res = await permission(member, [
			ruleset('lock', ['DisableDirectDeployment', 'RestrictDeployToDeployers'])
		])
		expect(res.reason).toContain('Direct deployment')
	})

	// The server refuses an operator in the item handler regardless of their global role: a
	// superadmin who is an operator in the workspace still gets 401 creating a script. So the
	// operator term has to stay above the admin/superadmin short-circuit — hoisting the admin
	// checks would offer a deploy the server refuses.
	it('refuses an operator who is also a superadmin', async () => {
		const res = await permission({ ...member, operator: true, is_super_admin: true }, [])
		expect(res.ok).toBe(false)
		expect(res.reason).toContain('operator')
	})
})

describe('scoping a refusal to the kinds the server gates', () => {
	const locked: DeployPermission = {
		ok: false,
		reason: 'Direct deployment to prod is disabled',
		refusedBy: 'DisableDirectDeployment'
	}
	const deployersOnly: DeployPermission = {
		ok: false,
		reason: 'Only workspace admins and members of wm_deployers can deploy to prod',
		refusedBy: 'RestrictDeployToDeployers'
	}

	// The kinds whose handlers call check_deploy_rules, against those that reach no gate.
	it('gates the item kinds the server gates, and no others', () => {
		for (const k of [
			'script',
			'flow',
			'app',
			'raw_app',
			'resource',
			'resource_type',
			'variable',
			'folder'
		] as const) {
			expect(kindGatedByDeployRules(k)).toBe(true)
		}
		for (const k of ['schedule', 'http_trigger', 'email_trigger', 'data_pipeline'] as const) {
			expect(kindGatedByDeployRules(k)).toBe(false)
		}
	})

	// Draft rows speak UserDraftItemKind; the gated names coincide, the ungated ones don't.
	it('reads draft kinds with the same lookup', () => {
		expect(kindGatedByDeployRules('variable')).toBe(true)
		expect(kindGatedByDeployRules('trigger_schedule')).toBe(false)
		expect(kindGatedByDeployRules('trigger_http')).toBe(false)
	})

	it('keeps a direct-deployment refusal off the kinds the server never gates', () => {
		expect(deployPermissionForKind(locked, 'script').ok).toBe(false)
		expect(deployPermissionForKind(locked, 'schedule').ok).toBe(true)
		expect(deployPermissionForKind(locked, 'trigger_schedule').ok).toBe(true)
	})

	// The deployers-only term over-reaches the same way, but it does so on main too. Narrowing it
	// would loosen the UI beyond mirroring the new rule, so it stays workspace-wide.
	it('leaves the deployers-only refusal applying to every kind', () => {
		expect(deployPermissionForKind(deployersOnly, 'script').ok).toBe(false)
		expect(deployPermissionForKind(deployersOnly, 'schedule').ok).toBe(false)
	})

	// `[].some()` is false, so an unguarded fold reports the empty selection as deployable and
	// drops the only message saying why the action is unavailable.
	it('keeps the refusal when nothing is selected', () => {
		expect(deployPermissionForKinds(locked, []).ok).toBe(false)
		expect(deployPermissionForKinds(deployersOnly, []).ok).toBe(false)
	})

	it('blocks a mixed selection but frees an all-ungated one', () => {
		expect(deployPermissionForKinds(locked, ['schedule', 'script']).ok).toBe(false)
		expect(deployPermissionForKinds(locked, ['schedule', 'http_trigger']).ok).toBe(true)
		expect(deployPermissionForKinds(deployersOnly, ['schedule']).ok).toBe(false)
	})
})

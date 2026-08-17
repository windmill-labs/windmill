import { beforeEach, describe, expect, it, vi } from 'vitest'

const { whoami, deployPermission } = vi.hoisted(() => ({
	whoami: vi.fn(),
	deployPermission: vi.fn()
}))

vi.mock('$lib/gen', () => ({ UserService: { whoami } }))
vi.mock('$lib/utils_workspace_deploy', () => ({ checkDeployPermission: deployPermission }))

import { resolveSessionAccess } from './sessionAccess'

type WhoamiOverrides = { is_admin?: boolean; is_super_admin?: boolean; operator?: boolean }

function user(overrides: WhoamiOverrides) {
	return {
		email: 'u@windmill.dev',
		username: 'u',
		is_admin: false,
		is_super_admin: false,
		operator: false,
		created_at: '',
		disabled: false,
		groups: [],
		folders: [],
		folders_read: [],
		folders_owners: [],
		...overrides
	}
}

async function capabilitiesFor(overrides: WhoamiOverrides, workspace = 'ws') {
	whoami.mockResolvedValueOnce(user(overrides))
	const access = await resolveSessionAccess(workspace)
	return access.capabilities
}

describe('resolveSessionAccess', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		deployPermission.mockResolvedValue({ ok: true })
	})

	it('gives a developer every capability', async () => {
		const caps = await capabilitiesFor({})
		expect([...caps].sort()).toEqual(['deploy', 'run_preview', 'write_draft'])
	})

	it('leaves an operator only what their token can still do', async () => {
		deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps]).toEqual([])
	})

	// The two gates have OPPOSITE precedence in the backend, so a role ladder would
	// get one of them wrong: drafts.rs returns Ok for `authed.is_admin` before the
	// operator branch, while jobs.rs checks the operator flag first with no escape.
	it('lets an admin who is also an operator write drafts but not run previews', async () => {
		deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
		const caps = await capabilitiesFor({ is_admin: true, operator: true })
		expect(caps.has('write_draft')).toBe(true)
		expect(caps.has('run_preview')).toBe(false)
	})

	// `ApiAuthed.is_admin` is `usr.is_admin || super_admin`, but whoami reports the two
	// separately — so a superadmin who is an operator must resolve the same way.
	it('treats a superadmin as an admin for the draft gate', async () => {
		deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
		const caps = await capabilitiesFor({ is_super_admin: true, operator: true })
		expect(caps.has('write_draft')).toBe(true)
		expect(caps.has('run_preview')).toBe(false)
	})

	// Deploy is operation-shaped: protection rulesets can block a plain developer, and
	// wm_deployers can unblock a non-admin. The resolver must not second-guess it.
	it('takes deploy from the shared permission check, not from the role', async () => {
		deployPermission.mockResolvedValue({ ok: false, reason: 'restricted to deployers' })
		const caps = await capabilitiesFor({})
		expect(caps.has('deploy')).toBe(false)
		expect(caps.has('write_draft')).toBe(true)
	})

	// checkDeployPermission tests `me.is_admin` alone, while the backend rule it mirrors
	// receives `authed.is_admin` (workspace admin OR superadmin). The resolver folds the
	// two before delegating, so a superadmin keeps deploy under RestrictDeployToDeployers.
	it('presents a superadmin as admin to the deploy check', async () => {
		whoami.mockResolvedValueOnce(user({ is_super_admin: true }))
		await resolveSessionAccess('ws')
		expect(deployPermission).toHaveBeenCalledWith('ws', expect.objectContaining({ is_admin: true }))
	})

	// Fail open, matching checkDeployPermission: a transient whoami failure must not
	// strip a session's toolset — the server is still the enforcement point.
	it('grants everything when the role cannot be resolved', async () => {
		whoami.mockRejectedValueOnce(new Error('network'))
		const access = await resolveSessionAccess('ws')
		expect(access.capabilities.has('write_draft')).toBe(true)
		expect(access.capabilities.has('deploy')).toBe(true)
	})
})

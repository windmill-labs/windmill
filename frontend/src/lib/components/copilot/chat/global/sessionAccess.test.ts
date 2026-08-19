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
		expect([...caps].sort()).toEqual([
			'deploy',
			'deploy_gated_kinds',
			'run_preview',
			'write_draft'
		])
	})

	it('leaves an operator only what their token can still do', async () => {
		deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps]).toEqual([])
	})

	// The two gates have OPPOSITE precedence in the backend, so a role ladder would
	// get one of them wrong: drafts.rs returns Ok for `authed.is_admin` before the
	// operator branch, while jobs.rs checks the operator flag first with no escape.
	// `authed.is_admin` is `usr.is_admin || super_admin`, so both spellings must land
	// on the same side of it.
	it.each([{ is_admin: true }, { is_super_admin: true }])(
		'lets an admin who is also an operator write drafts but not run previews (%o)',
		async (role) => {
			deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
			const caps = await capabilitiesFor({ ...role, operator: true })
			expect(caps.has('write_draft')).toBe(true)
			expect(caps.has('run_preview')).toBe(false)
		}
	)

	// Deploy is operation-shaped: protection rulesets can block a plain developer, and
	// wm_deployers can unblock a non-admin. The resolver must not second-guess the shared
	// preflight, and must not let a deploy refusal take drafting down with it.
	it('takes deploy from the shared permission check, not from the role', async () => {
		deployPermission.mockResolvedValue({
			ok: false,
			reason: 'restricted to deployers',
			refusedBy: 'RestrictDeployToDeployers'
		})
		const caps = await capabilitiesFor({})
		expect(caps.has('deploy')).toBe(false)
		expect(caps.has('deploy_gated_kinds')).toBe(false)
		expect(caps.has('write_draft')).toBe(true)
	})

	// A direct-deployment lock is the one refusal that does not cover every kind: the
	// server still accepts schedule and trigger deploys, so the resolver must keep the
	// weaker half rather than dropping deploy wholesale.
	it('keeps the ungated half of deploy under a direct-deployment lock', async () => {
		deployPermission.mockResolvedValue({
			ok: false,
			reason: 'direct deployment disabled',
			refusedBy: 'DisableDirectDeployment'
		})
		const caps = await capabilitiesFor({})
		expect(caps.has('deploy')).toBe(true)
		expect(caps.has('deploy_gated_kinds')).toBe(false)
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

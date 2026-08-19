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

	// Pins the per-capability precedence documented in resolveSessionAccess, for both
	// spellings of `authed.is_admin`.
	it.each([{ is_admin: true }, { is_super_admin: true }])(
		'lets an admin who is also an operator write drafts but not run previews (%o)',
		async (role) => {
			deployPermission.mockResolvedValue({ ok: false, reason: 'operators cannot deploy' })
			const caps = await capabilitiesFor({ ...role, operator: true })
			expect(caps.has('write_draft')).toBe(true)
			expect(caps.has('run_preview')).toBe(false)
		}
	)

	// A deploy refusal must not take drafting down with it.
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

	// The one refusal that does not cover every kind.
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

	it('grants everything when the role cannot be resolved', async () => {
		whoami.mockRejectedValueOnce(new Error('network'))
		const access = await resolveSessionAccess('ws')
		expect(access.capabilities.has('write_draft')).toBe(true)
		expect(access.capabilities.has('deploy')).toBe(true)
	})
})

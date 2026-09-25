import { beforeEach, describe, expect, it, vi } from 'vitest'

const { whoami, deployRules } = vi.hoisted(() => ({
	whoami: vi.fn(),
	deployRules: vi.fn()
}))

vi.mock('$lib/gen', () => ({ UserService: { whoami } }))
vi.mock('$lib/utils_workspace_deploy', () => ({ checkDeployRules: deployRules }))

import { resolveSessionAccess } from './sessionAccess'
import { clearWorkspaceRoleCache } from '$lib/user'

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
	return await resolveSessionAccess(workspace)
}

describe('resolveSessionAccess', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		deployRules.mockResolvedValue({ ok: true })
		// The role memo is app-wide, so without this each case answers from the previous one.
		clearWorkspaceRoleCache()
	})

	it('gives a developer every capability but admin', async () => {
		const caps = await capabilitiesFor({})
		expect([...caps].sort()).toEqual(['deploy', 'manage_code', 'run_preview', 'write_draft'])
	})

	// The folder, resource and variable handlers run the rules but have no operator check,
	// so withholding `deploy` from an operator would be stricter than the server.
	it('leaves an operator the deploy-rule capability, and nothing their token refuses', async () => {
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps]).toEqual(['deploy'])
	})

	it('takes deploy from the rules alone, so a rule blocks an operator too', async () => {
		deployRules.mockResolvedValue({ ok: false, refusedBy: 'DisableDirectDeployment' })
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps]).toEqual([])
	})

	// Both spellings of `authed.is_admin`, which the draft path honours and the handlers
	// refusing `authed.is_operator` do not — the session path never clears that flag.
	it.each([{ is_admin: true }, { is_super_admin: true }])(
		'lets an admin who is also an operator draft and deploy, but not preview or manage code (%o)',
		async (role) => {
			const caps = await capabilitiesFor({ ...role, operator: true })
			expect([...caps].sort()).toEqual(['admin', 'deploy', 'write_draft'])
		}
	)

	it('keeps drafting when a rule refuses deploy', async () => {
		deployRules.mockResolvedValue({
			ok: false,
			reason: 'restricted to deployers',
			refusedBy: 'RestrictDeployToDeployers'
		})
		const caps = await capabilitiesFor({})
		expect(caps.has('deploy')).toBe(false)
		expect(caps.has('write_draft')).toBe(true)
	})

	it('grants everything when the role cannot be resolved', async () => {
		whoami.mockRejectedValueOnce(new Error('network'))
		const access = await resolveSessionAccess('ws')
		expect(access.has('write_draft')).toBe(true)
		expect(access.has('deploy')).toBe(true)
	})
})

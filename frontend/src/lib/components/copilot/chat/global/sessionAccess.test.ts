import { beforeEach, describe, expect, it, vi } from 'vitest'

const { whoami, deployRules, protectionRules } = vi.hoisted(() => ({
	whoami: vi.fn(),
	deployRules: vi.fn(),
	protectionRules: vi.fn()
}))

vi.mock('$lib/gen', () => ({ UserService: { whoami } }))
vi.mock('$lib/utils_workspace_deploy', () => ({ checkDeployRules: deployRules }))
vi.mock('$lib/workspaceProtectionRules.svelte', () => ({
	fetchProtectionRulesForWorkspace: protectionRules,
	isRuleActiveInRulesets: (rulesets: { rules: string[] }[], kind: string) =>
		rulesets.some((r) => r.rules.includes(kind))
}))

import { capabilitiesForRole, resolveSessionAccess } from './sessionAccess'
import { clearWorkspaceRoleCache } from '$lib/user'
import { FULL_CAPABILITIES, legacyOperatorCapabilities } from '$lib/roleCapabilities'
import type { OperatorSettings, RoleCapabilities } from '$lib/gen'

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
		protectionRules.mockResolvedValue([])
		// The role memo is app-wide, so without this each case answers from the previous one.
		clearWorkspaceRoleCache()
	})

	it('gives a developer every capability but admin', async () => {
		const caps = await capabilitiesFor({})
		expect([...caps].sort()).toEqual([
			'deploy',
			'manage_apps',
			'manage_code',
			'manage_flows',
			'manage_schedules',
			'manage_triggers',
			'modify_workspace',
			'run_flow_preview',
			'run_preview',
			'write_app_draft',
			'write_draft',
			'write_flow_draft',
			'write_inline_app_code',
			'write_inline_flow_code',
			'write_resource_draft',
			'write_schedule_draft',
			'write_trigger_draft',
			'write_variable_draft'
		])
	})

	// The folder, resource and variable handlers run the rules but have no operator check,
	// so withholding `deploy` from an operator would be stricter than the server.
	it('leaves an operator the deploy-rule capability, and nothing their token refuses', async () => {
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps].sort()).toEqual([
			'deploy',
			'manage_schedules',
			'manage_triggers',
			'modify_workspace',
			'write_resource_draft',
			'write_schedule_draft',
			'write_trigger_draft',
			'write_variable_draft'
		])
	})

	it('takes deploy from the rules alone, so a rule blocks an operator too', async () => {
		deployRules.mockResolvedValue({ ok: false, refusedBy: 'DisableDirectDeployment' })
		const caps = await capabilitiesFor({ operator: true })
		expect([...caps].sort()).toEqual([
			'manage_schedules',
			'manage_triggers',
			'modify_workspace',
			'write_resource_draft',
			'write_schedule_draft',
			'write_trigger_draft',
			'write_variable_draft'
		])
	})

	// Both spellings of `authed.is_admin`: `caller_capabilities` gives an admin every role
	// capability, whatever role the membership row names.
	it.each([{ is_admin: true }, { is_super_admin: true }])(
		'gives an admin whose role is operator everything (%o)',
		async (role) => {
			const caps = await capabilitiesFor({ ...role, operator: true })
			expect([...caps].sort()).toEqual([
				'admin',
				'deploy',
				'manage_apps',
				'manage_code',
				'manage_flows',
				'manage_schedules',
				'manage_triggers',
				'modify_workspace',
				'run_flow_preview',
				'run_preview',
				'write_app_draft',
				'write_draft',
				'write_flow_draft',
				'write_inline_app_code',
				'write_inline_flow_code',
				'write_resource_draft',
				'write_schedule_draft',
				'write_trigger_draft',
				'write_variable_draft'
			])
		}
	)

	// A production workspace: the session reads and runs, and holds nothing that writes, even
	// for an admin the rule would let through.
	it('makes the session read-only in a workspace protected by DisableDirectDeployment', async () => {
		protectionRules.mockResolvedValue([{ rules: ['DisableDirectDeployment'] }])
		const caps = await capabilitiesFor({ is_admin: true })
		expect([...caps]).toEqual([])
	})

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

	// flows.rs, drafts.rs and run_preview_flow all let a builder through for flows, and
	// nothing else moves: scripts, apps and their previews stay refused.
	it("gives a builder operator the flow capabilities and nothing a script's handler refuses", () => {
		const caps = capabilitiesForRole({
			isAdmin: false,
			deployRulesPass: true,
			capabilities: legacyOperatorCapabilities({ builder_flows: true } as OperatorSettings)
		})
		expect([...caps].sort()).toEqual([
			'deploy',
			'manage_flows',
			'manage_schedules',
			'manage_triggers',
			'modify_workspace',
			'run_flow_preview',
			'write_flow_draft',
			'write_resource_draft',
			'write_schedule_draft',
			'write_trigger_draft',
			'write_variable_draft'
		])
	})

	// apps.rs and drafts.rs let an app builder through; nothing runs an app preview server-side.
	it('gives an app builder operator the app capabilities alone', () => {
		const caps = capabilitiesForRole({
			isAdmin: false,
			deployRulesPass: true,
			capabilities: legacyOperatorCapabilities({ builder_apps: true } as OperatorSettings)
		})
		expect([...caps].sort()).toEqual([
			'deploy',
			'manage_apps',
			'manage_schedules',
			'manage_triggers',
			'modify_workspace',
			'write_app_draft',
			'write_resource_draft',
			'write_schedule_draft',
			'write_trigger_draft',
			'write_variable_draft'
		])
	})

	// A withdrawable right: only `false` withdraws it, as `gate_operator_writes` reads it.
	it('withholds schedule and trigger writes only where the workspace set them to false', () => {
		const caps = capabilitiesForRole({
			isAdmin: false,
			deployRulesPass: true,
			capabilities: legacyOperatorCapabilities({ manage_schedules: false } as OperatorSettings)
		})
		expect(caps.has('manage_schedules')).toBe(false)
		expect(caps.has('write_schedule_draft')).toBe(false)
		expect(caps.has('manage_triggers')).toBe(true)
		expect(caps.has('write_trigger_draft')).toBe(true)
	})

	// A role that codes but builds no apps: scripts and flows, and nothing apps.rs refuses.
	it('follows the capabilities of a custom role rather than the operator flag', () => {
		const scripter: RoleCapabilities = { ...FULL_CAPABILITIES, apps: 'none' }
		const caps = capabilitiesForRole({
			isAdmin: false,
			deployRulesPass: true,
			capabilities: scripter
		})
		expect(caps.has('manage_code')).toBe(true)
		expect(caps.has('manage_flows')).toBe(true)
		expect(caps.has('manage_apps')).toBe(false)
		expect(caps.has('write_app_draft')).toBe(false)
	})

	it('grants everything when the role cannot be resolved', async () => {
		whoami.mockRejectedValueOnce(new Error('network'))
		const access = await resolveSessionAccess('ws')
		expect(access.has('write_draft')).toBe(true)
		expect(access.has('deploy')).toBe(true)
	})
})

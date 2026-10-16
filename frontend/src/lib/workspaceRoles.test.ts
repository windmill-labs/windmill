import { describe, it, expect } from 'vitest'
import type { EffectiveRole } from '$lib/gen'
import {
	applyCapabilitiesPatch,
	baseCapabilitiesFor,
	FULL_CAPABILITIES,
	validRoleOr,
	workspaceRolesErrors
} from './workspaceRoles'

// These mirror `windmill_common::roles`; the server refuses what they let through.
describe('role capability layering', () => {
	it('falls back to full for developer and to the operator rights for any other name', () => {
		expect(baseCapabilitiesFor('developer')).toEqual(FULL_CAPABILITIES)
		const other = baseCapabilitiesFor('reviewer')
		expect([other.code, other.flows, other.manage_schedules]).toEqual([false, 'none', true])
	})

	it('applies only the keys a patch sets', () => {
		const caps = applyCapabilitiesPatch(baseCapabilitiesFor('x'), {
			flows: 'compose',
			visibility: { workers: false }
		})
		expect(caps.flows).toBe('compose')
		expect(caps.apps).toBe('none')
		expect(caps.visibility.workers).toBe(false)
		expect(caps.visibility.runs).toBe(true)
	})

	it('treats a stored null as unset, as serde reads it', () => {
		const caps = applyCapabilitiesPatch(baseCapabilitiesFor('developer'), {
			code: null as any,
			visibility: { runs: null as any }
		})
		expect(caps).toEqual(FULL_CAPABILITIES)
		expect(
			workspaceRolesErrors([{ name: 'developer', capabilities: { code: null as any } }])
		).toEqual({})
	})

	it('refuses what parse_instance_roles refuses', () => {
		expect(workspaceRolesErrors(undefined)).toEqual({})
		expect(workspaceRolesErrors([])).toHaveProperty('list')
		expect(workspaceRolesErrors([{ name: 'admin' }])).toHaveProperty('0.name')
		expect(workspaceRolesErrors([{ name: 'a' }, { name: 'a' }])).toHaveProperty('1.name')
		expect(workspaceRolesErrors([{ name: 'x', capabilities: { flows: 'full' } }])).toHaveProperty(
			'0.capabilities'
		)
		expect(
			workspaceRolesErrors([{ name: 'developer', capabilities: { apps: 'compose' } }])
		).toEqual({})
	})
})

describe('validRoleOr', () => {
	const role = (name: string) => ({ name }) as EffectiveRole

	it('falls back to developer, or the first role when the instance dropped it', () => {
		expect(validRoleOr([role('operator'), role('developer')], undefined)).toBe('developer')
		expect(validRoleOr([role('builder'), role('viewer')], undefined)).toBe('builder')
		expect(validRoleOr([role('builder'), role('viewer')], 'developer')).toBe('builder')
		expect(validRoleOr([role('builder')], 'admin')).toBe('admin')
	})
})

import { describe, it, expect } from 'vitest'
import { deployedPermissionedAs, keepsDeployedOnBehalfOf } from './onBehalfOf'

// Who a deploy leaves the item running on behalf of: only admins and wm_deployers keep another user.
describe('onBehalfOf', () => {
	const admin = { username: 'admin', is_admin: true }
	const deployer = { username: 'dep', groups: ['wm_deployers'] }
	const member = { username: 'alice' }

	it('keeps the held user only for a deployer who may preserve', () => {
		expect(deployedPermissionedAs(admin, 'u/bot')).toBe('u/bot')
		expect(deployedPermissionedAs(deployer, 'u/bot')).toBe('u/bot')
		expect(deployedPermissionedAs(admin, undefined)).toBe('u/admin')
		expect(deployedPermissionedAs(member, 'u/bot')).toBe('u/alice')
	})

	it('picks the deployed user only when nothing is picked and run-as is on', () => {
		expect(keepsDeployedOnBehalfOf(admin, undefined, 'bot@x.dev', 'bot@x.dev')).toBe(true)
		expect(keepsDeployedOnBehalfOf(deployer, undefined, 'me@x.dev', 'bot@x.dev')).toBe(true)
		expect(keepsDeployedOnBehalfOf(admin, 'me', 'me@x.dev', 'bot@x.dev')).toBe(false)
		expect(keepsDeployedOnBehalfOf(admin, undefined, undefined, 'bot@x.dev')).toBe(false)
		expect(keepsDeployedOnBehalfOf(admin, undefined, 'me@x.dev', undefined)).toBe(false)
		expect(keepsDeployedOnBehalfOf(member, undefined, 'bot@x.dev', 'bot@x.dev')).toBe(false)
	})
})

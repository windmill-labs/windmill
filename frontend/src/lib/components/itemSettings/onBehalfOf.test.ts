import { describe, it, expect } from 'vitest'
import {
	appDeployAsksOnBehalfOf,
	deployedPermissionedAs,
	preserveOnBehalfOfOnDeploy
} from './onBehalfOf'

// The editors show, before a deploy, who the backend will leave the item running on behalf of,
// and the app deploy dialog asks from the same answer.
describe('onBehalfOf', () => {
	const admin = { username: 'admin', is_admin: true }
	const deployer = { username: 'dep', groups: ['wm_deployers'] }
	const member = { username: 'alice' }

	it('keeps the picked user only for a deployer who may preserve and asks to', () => {
		expect(deployedPermissionedAs(admin, 'u/bot', true)).toBe('u/bot')
		expect(deployedPermissionedAs(deployer, 'u/bot', true)).toBe('u/bot')
		expect(deployedPermissionedAs(member, 'u/bot', true)).toBe('u/alice')
		expect(deployedPermissionedAs(admin, 'u/bot', undefined)).toBe('u/admin')
	})

	it('keeps the user the editor holds until the settings pick one', () => {
		expect(preserveOnBehalfOfOnDeploy(undefined, true)).toBe(true)
		expect(preserveOnBehalfOfOnDeploy(undefined, false)).toBe(undefined)
		expect(preserveOnBehalfOfOnDeploy(false, true)).toBe(undefined)
	})

	it('asks on the first deploy of an app and on any deploy that changes its user', () => {
		expect(appDeployAsksOnBehalfOf(true, undefined, 'u/admin')).toBe(true)
		expect(appDeployAsksOnBehalfOf(false, 'u/bot', 'u/bot')).toBe(false)
		expect(appDeployAsksOnBehalfOf(false, 'u/bot', 'u/admin')).toBe(true)
	})
})

import { describe, it, expect } from 'vitest'
import { deployAsksOnBehalfOf, deployedPermissionedAs } from './onBehalfOf'

// The deploy dialog names who the backend will leave the item running on behalf of, and asks only
// when that differs from the deployed version.
describe('onBehalfOf', () => {
	const admin = { username: 'admin', email: 'admin@x.dev', is_admin: true }
	const deployer = { username: 'dep', groups: ['wm_deployers'] }
	const member = { username: 'alice', email: 'alice@x.dev' }

	it('keeps the held user, then the folder default, only for a deployer who may preserve', () => {
		expect(deployedPermissionedAs(admin, 'u/bot', 'u/folder')).toBe('u/bot')
		expect(deployedPermissionedAs(deployer, undefined, 'u/folder')).toBe('u/folder')
		expect(deployedPermissionedAs(admin, undefined, undefined)).toBe('u/admin')
		expect(deployedPermissionedAs(member, 'u/bot', 'u/folder')).toBe('u/alice')
	})

	it('asks on a first deploy, and on deploys that change the run-as user', () => {
		const none = {
			email: undefined,
			permissionedAs: undefined,
			savedEmail: undefined,
			savedPermissionedAs: undefined
		}
		const bot = { email: 'bot@x.dev', permissionedAs: 'u/bot' }
		const deployedAsBot = { savedEmail: 'bot@x.dev', savedPermissionedAs: 'u/bot' }

		expect(deployAsksOnBehalfOf('flow', { user: admin, ...none })).toBe(false)
		// An app runs on behalf of someone from its first deploy.
		expect(deployAsksOnBehalfOf('app', { user: admin, ...none })).toBe(true)
		expect(deployAsksOnBehalfOf('app', { user: admin, ...bot, ...deployedAsBot })).toBe(false)
		expect(deployAsksOnBehalfOf('app', { user: member, ...bot, ...deployedAsBot })).toBe(true)
		// A first deploy, or one that turns it on.
		expect(deployAsksOnBehalfOf('script', { user: admin, ...none, ...bot })).toBe(true)
		expect(deployAsksOnBehalfOf('flow', { user: admin, ...none, folderDefault: 'u/f' })).toBe(true)
		// Same user as deployed.
		expect(deployAsksOnBehalfOf('flow', { user: admin, ...bot, ...deployedAsBot })).toBe(false)
		// Stops.
		expect(deployAsksOnBehalfOf('flow', { user: admin, ...none, ...deployedAsBot })).toBe(true)
		// A script renamed into a folder takes its default over the deployed user.
		expect(
			deployAsksOnBehalfOf('script', {
				user: admin,
				...none,
				...deployedAsBot,
				folderDefault: 'u/f'
			})
		).toBe(true)
		// The backend replaces the user with a deployer who may not preserve it.
		expect(deployAsksOnBehalfOf('flow', { user: member, ...bot, ...deployedAsBot })).toBe(true)
	})
})

import { describe, it, expect } from 'vitest'
import {
	deployAsksOnBehalfOf,
	deployOnBehalfOfQuestion,
	deployedPermissionedAs,
	setDeployedOnBehalfOf
} from './onBehalfOf'

// The deploy dialog names who the backend will leave the item running on behalf of, and the
// app editors keep that answer as the deployed policy.
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

	it('asks on every app deploy, and on flow or script deploys that involve a run-as user', () => {
		expect(deployAsksOnBehalfOf('app', false, false)).toBe(true)
		expect(deployAsksOnBehalfOf('flow', false, false)).toBe(false)
		expect(deployAsksOnBehalfOf('script', true, false)).toBe(true)
		expect(deployAsksOnBehalfOf('flow', false, true)).toBe(true)
		expect(deployAsksOnBehalfOf('flow', false, false, 'u/folder')).toBe(true)
	})

	it('writes the stored user into the policy, with the email only for the deployer', () => {
		const forMember = { on_behalf_of: 'u/bot', on_behalf_of_email: 'bot@x.dev' }
		setDeployedOnBehalfOf(forMember, member)
		expect(forMember).toEqual({ on_behalf_of: 'u/alice', on_behalf_of_email: 'alice@x.dev' })

		const newInFolder: { on_behalf_of?: string; on_behalf_of_email?: string } = {}
		setDeployedOnBehalfOf(newInFolder, admin, 'u/folder')
		expect(newInFolder).toEqual({ on_behalf_of: 'u/folder', on_behalf_of_email: undefined })
	})

	it('asks to run on behalf of someone or to stop, and says when the user is the deployed one', () => {
		const held = {
			email: undefined,
			permissionedAs: undefined,
			savedEmail: undefined,
			savedPermissionedAs: undefined,
			isNew: true
		}
		expect(deployOnBehalfOfQuestion('app', admin, held).title).toBe(
			'Run this app on behalf of u/admin (you)?'
		)
		expect(
			deployOnBehalfOfQuestion('app', admin, { ...held, folderDefault: 'u/folder' }).title
		).toBe('Run this app on behalf of u/folder?')
		expect(
			deployOnBehalfOfQuestion('flow', admin, {
				...held,
				savedPermissionedAs: 'u/bot',
				isNew: false
			})
		).toMatchObject({ stops: true, title: 'Stop running this flow on behalf of u/bot?' })
		expect(
			deployOnBehalfOfQuestion('flow', admin, {
				...held,
				email: 'bot@x.dev',
				permissionedAs: 'u/bot',
				savedPermissionedAs: 'u/bot',
				isNew: false
			})
		).toMatchObject({ stops: false, runsAs: 'u/bot', unchanged: true })
		expect(deployOnBehalfOfQuestion('flow', admin, held)).toMatchObject({
			required: false,
			on: false,
			title: 'Deploy this flow?'
		})
	})
})

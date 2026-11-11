import { describe, it, expect } from 'vitest'
import { setDeployedOnBehalfOf } from './appDeploy.svelte'

// The app editors keep the policy they sent as the deployed one.
describe('setDeployedOnBehalfOf', () => {
	const admin = { username: 'admin', email: 'admin@x.dev', is_admin: true }
	const member = { username: 'alice', email: 'alice@x.dev' }

	it('writes the stored user into the policy, with the email only for the deployer', () => {
		const forMember = { on_behalf_of: 'u/bot', on_behalf_of_email: 'bot@x.dev' }
		setDeployedOnBehalfOf(forMember, member)
		expect(forMember).toEqual({ on_behalf_of: 'u/alice', on_behalf_of_email: 'alice@x.dev' })

		const newInFolder: { on_behalf_of?: string; on_behalf_of_email?: string } = {}
		setDeployedOnBehalfOf(newInFolder, admin, 'u/folder')
		expect(newInFolder).toEqual({ on_behalf_of: 'u/folder', on_behalf_of_email: undefined })
	})
})

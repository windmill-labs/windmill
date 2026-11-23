import { describe, expect, it, vi } from 'vitest'

const services = vi.hoisted(() => ({
	existsVariable: vi.fn(),
	createVariable: vi.fn(),
	updateVariable: vi.fn(),
	existsResource: vi.fn(),
	createResource: vi.fn(),
	updateResource: vi.fn()
}))
vi.mock('$lib/gen', () => ({
	VariableService: {
		existsVariable: services.existsVariable,
		createVariable: services.createVariable,
		updateVariable: services.updateVariable
	},
	ResourceService: {
		existsResource: services.existsResource,
		createResource: services.createResource,
		updateResource: services.updateResource
	}
}))
vi.mock('$app/paths', () => ({ base: '' }))
vi.mock('$lib/components/sessions/sessionMode.svelte', () => ({
	withWorkspaceParam: (url: string) => url
}))

import { connectionStringParts, saveSecretRequest, secretRequestToolResult } from './secretRequest'
import type { SecretRequest } from './shared'

const bare: SecretRequest = { workspace: 'w', path: 'u/a/key', label: 'API key' }
const connect: SecretRequest = {
	workspace: 'w',
	path: 'u/a/acme',
	label: 'Acme',
	resource: {
		type: 'acme',
		value: { host: 'acme.io' },
		secretFields: [
			{ field: 'client_id', label: 'Client ID' },
			{ field: 'client_secret', label: 'Client secret' }
		]
	}
}

// The model reads this back: it must name where the secret is, and nothing else of it.
describe('secretRequestToolResult', () => {
	it('gives the path the user saved to, and says so when it is not the one asked for', () => {
		const same = JSON.parse(
			secretRequestToolResult(bare, { path: 'u/a/key', replaced: false, variables: ['u/a/key'] })
		)
		expect(same).toMatchObject({ success: true, variable: '$var:u/a/key' })
		expect(same.message).not.toContain('instead of')

		const moved = JSON.parse(
			secretRequestToolResult(connect, {
				path: 'u/a/acme_2',
				replaced: false,
				variables: ['u/a/acme_2_client_id', 'u/a/acme_2_client_secret']
			})
		)
		expect(moved.resource).toBe('$res:u/a/acme_2')
		expect(moved.message).toContain('instead of u/a/acme')
	})

	it('reports a dismissal as a failure with no variable', () => {
		const dismissed = JSON.parse(secretRequestToolResult(bare, undefined))
		expect(dismissed.success).toBe(false)
		expect(dismissed.variable).toBeUndefined()
	})
})

describe('saveSecretRequest', () => {
	it('saves each secret, then the resource referencing them, and retries over its own variables', async () => {
		services.existsResource.mockResolvedValue(false)
		services.existsVariable.mockResolvedValue(false)
		services.createResource.mockRejectedValueOnce(new Error('refused'))
		const own = new Set<string>()
		const submission = {
			path: 'u/a/acme',
			replace: false,
			secrets: { client_id: 'id', client_secret: 'shh' },
			fields: { host: 'eu.acme.io' }
		}
		await expect(saveSecretRequest(connect, submission, own)).rejects.toThrow('refused')

		// The variables are there now: a retry updates them rather than calling the path taken.
		services.existsVariable.mockResolvedValue(true)
		const outcome = await saveSecretRequest(connect, submission, own)
		expect(outcome.variables).toEqual(['u/a/acme_client_id', 'u/a/acme_client_secret'])
		expect(services.updateVariable).toHaveBeenCalledTimes(2)
		expect(services.createResource).toHaveBeenLastCalledWith({
			workspace: 'w',
			requestBody: expect.objectContaining({
				resource_type: 'acme',
				value: {
					host: 'eu.acme.io',
					client_id: '$var:u/a/acme_client_id',
					client_secret: '$var:u/a/acme_client_secret'
				}
			})
		})
	})
})

// The password goes to a secret variable; everything else must be readable on the card.
describe('connectionStringParts', () => {
	it('splits out the password and defaults sslmode as the resource editor does', () => {
		expect(connectionStringParts(' postgres://app:p%40ss@db.example.com:6432/orders ')).toEqual({
			secret: 'p@ss',
			fields: {
				user: 'app',
				host: 'db.example.com',
				port: 6432,
				dbname: 'orders',
				sslmode: 'require'
			}
		})
	})

	it('refuses a string it cannot store faithfully', () => {
		expect(connectionStringParts('mysql://a:b@h/d')).toHaveProperty('error')
		expect(connectionStringParts('postgres://app@h/d')).toHaveProperty('error')
		expect(
			connectionStringParts('postgres://a:b@h/d?target_session_attrs=read-write')
		).toHaveProperty('error')
	})
})

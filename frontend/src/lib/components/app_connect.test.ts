import { describe, it, expect } from 'vitest'
import { detectClientCredentials } from './app_connect'

function schema(props: Record<string, { type?: string }>) {
	return { type: 'object', properties: props }
}

describe('detectClientCredentials', () => {
	it('detects snake_case credential fields', () => {
		expect(
			detectClientCredentials(
				schema({
					client_id: { type: 'string' },
					client_secret: { type: 'string' },
					token: { type: 'string' }
				})
			)
		).toEqual({
			clientIdField: 'client_id',
			clientSecretField: 'client_secret',
			tokenField: 'token',
			tokenUrlField: undefined
		})
	})

	it('detects camelCase and prefixed fields (azure-style)', () => {
		expect(
			detectClientCredentials(
				schema({
					azureClientId: { type: 'string' },
					azureClientSecret: { type: 'string' },
					token: { type: 'string' }
				})
			)
		).toEqual({
			clientIdField: 'azureClientId',
			clientSecretField: 'azureClientSecret',
			tokenField: 'token',
			tokenUrlField: undefined
		})
	})

	it('picks up an unambiguous token URL field', () => {
		expect(
			detectClientCredentials(
				schema({
					client_id: { type: 'string' },
					client_secret: { type: 'string' },
					tokenUrl: { type: 'string' },
					token: { type: 'string' }
				})
			)?.tokenUrlField
		).toBe('tokenUrl')
	})

	it('rejects ambiguous client id candidates', () => {
		expect(
			detectClientCredentials(
				schema({
					client_id: { type: 'string' },
					backup_client_id: { type: 'string' },
					client_secret: { type: 'string' },
					token: { type: 'string' }
				})
			)
		).toBeUndefined()
	})

	it('requires a token field', () => {
		expect(
			detectClientCredentials(
				schema({
					client_id: { type: 'string' },
					client_secret: { type: 'string' }
				})
			)
		).toBeUndefined()
	})

	it('ignores non-string properties', () => {
		expect(
			detectClientCredentials(
				schema({
					client_id: { type: 'object' },
					client_secret: { type: 'string' },
					token: { type: 'string' }
				})
			)
		).toBeUndefined()
	})

	it('handles missing or malformed schemas', () => {
		expect(detectClientCredentials(undefined)).toBeUndefined()
		expect(detectClientCredentials({})).toBeUndefined()
		expect(detectClientCredentials({ properties: 'nope' })).toBeUndefined()
	})
})

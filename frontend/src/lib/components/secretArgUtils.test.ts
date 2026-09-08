import { beforeEach, describe, expect, it, vi } from 'vitest'

const created: { path: string; value: string; is_secret?: boolean }[] = []

vi.mock('$lib/gen', () => ({
	VariableService: {
		createVariable: vi.fn(async ({ requestBody }: any) => {
			created.push(requestBody)
			return requestBody.path
		})
	}
}))

vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { workspaceStore: writable('test-ws'), userStore: writable({ username: 'ada' }) }
})

import { processSecretArgs } from './secretArgUtils'

describe('processSecretArgs', () => {
	beforeEach(() => (created.length = 0))

	const schema = {
		properties: {
			token: { type: 'string', password: true },
			creds: { type: 'object', password: true, properties: { user: { type: 'string' } } },
			nested: { type: 'object', properties: { inner: { type: 'string', password: true } } },
			plain: { type: 'string' }
		}
	} as any

	// Nothing else turns a proposed secret into a reference when no form mounts, so a literal
	// left alone here is a plaintext credential stored on the job for anyone who can see it.
	it('mints a reference for a literal at every level, leaving other arguments alone', async () => {
		const out = await processSecretArgs(
			{ token: 'hunter2', creds: { user: 'ada' }, nested: { inner: 'deep' }, plain: 'kept' },
			schema
		)
		expect(out.token).toMatch(/^\$var:u\/ada\/secret_arg\//)
		expect(out.creds).toMatch(/^\$jsonvar:u\/ada\/secret_arg\//)
		expect(out.nested.inner).toMatch(/^\$var:u\/ada\/secret_arg\//)
		expect(out.plain).toBe('kept')
		// The object goes into the variable as JSON, which is what `$jsonvar:` parses back.
		expect(created.map((c) => c.value).sort()).toEqual(['deep', 'hunter2', '{"user":"ada"}'])
		expect(created.every((c) => c.is_secret)).toBe(true)
	})

	it('leaves a reference the caller already named alone', async () => {
		const out = await processSecretArgs(
			{ token: '$var:f/team/api_token', creds: '$jsonvar:u/ada/existing' },
			schema
		)
		expect(out).toEqual({ token: '$var:f/team/api_token', creds: '$jsonvar:u/ada/existing' })
		expect(created).toEqual([])
	})

	// `$var:` hands the job the variable's text; a field declaring an object needs it parsed,
	// which is the same variable read the other way rather than a secret the caller cannot see.
	it('reads a plain variable as JSON where the field cannot hold a string', async () => {
		const out = await processSecretArgs({ creds: '$var:u/ada/stripe' }, schema)
		expect(out.creds).toBe('$jsonvar:u/ada/stripe')
		expect(created).toEqual([])
	})

	it('leaves an absent or null secret absent', async () => {
		const out = await processSecretArgs({ token: null, plain: 'kept' }, schema)
		expect(out).toEqual({ token: null, plain: 'kept' })
		expect(created).toEqual([])
	})

	// A property name can itself contain a dot. Reported under one label these two leaves
	// would share a mint, and the flat field would run on the nested field's secret.
	it("tells apart a key that spells another key's path", async () => {
		const out = await processSecretArgs({ 'db.password': 'FLAT', db: { password: 'NESTED' } }, {
			properties: {
				'db.password': { type: 'string', password: true },
				db: {
					type: 'object',
					properties: { password: { type: 'string', password: true } }
				}
			}
		} as any)
		const flat = out['db.password'].slice('$var:'.length)
		const nested = out.db.password.slice('$var:'.length)
		expect(flat).not.toBe(nested)
		expect(created.find((c) => c.path === flat)?.value).toBe('FLAT')
		expect(created.find((c) => c.path === nested)?.value).toBe('NESTED')
	})
})

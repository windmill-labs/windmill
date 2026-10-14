import { describe, expect, it, vi } from 'vitest'

const request = vi.fn()
vi.mock('$lib/gen/core/request', () => ({ request: (...args: unknown[]) => request(...args) }))
vi.mock('$lib/gen', () => ({ OpenAPI: {} }))
vi.mock('$lib/cloud', () => ({ isCloudHosted: () => false }))

import { listUsableDatatableRoles } from './datatableUsableRoles'

const refuse = (body: string) => () =>
	Promise.reject(Object.assign(new Error('Bad Request'), { body }))

describe('listUsableDatatableRoles', () => {
	// The refusal is recognised by its sentence, so rewording it on one side alone would turn every
	// data table on a build without the Enterprise Edition into a failed lookup rather than one
	// that is not under roles.
	it('reads the enterprise refusal as not under roles, and rethrows anything else', async () => {
		request.mockImplementation(refuse('Data table roles are a Windmill Enterprise Edition feature'))
		expect(await listUsableDatatableRoles('ws', 'main')).toEqual({
			permissioned: false,
			roles: [],
			default_role: 'admin'
		})

		request.mockImplementation(refuse('Data table not found'))
		const rethrown = await listUsableDatatableRoles('ws', 'main').catch((e) => e)
		expect(rethrown).toBeInstanceOf(Error)
	})
})

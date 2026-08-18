import { beforeEach, describe, expect, it, vi } from 'vitest'

const { existsVariable, getVariable, createVariable, updateVariable, deleteVariable, existsResource } =
	vi.hoisted(() => ({
		existsVariable: vi.fn(),
		getVariable: vi.fn(),
		createVariable: vi.fn(),
		updateVariable: vi.fn(),
		deleteVariable: vi.fn(),
		existsResource: vi.fn()
	}))

vi.mock('$lib/gen', () => ({
	VariableService: { existsVariable, getVariable, createVariable, updateVariable, deleteVariable },
	ResourceService: { existsResource }
}))

import { upsertSecretVariable } from './secretVariable'

const OURS = 'MCP connection token for u/hugo/github_mcp'
const ARGS = {
	workspace: 'ws',
	path: 'u/hugo/github_mcp_token',
	value: 'token',
	resourcePath: 'u/hugo/github_mcp'
}

beforeEach(() => {
	vi.clearAllMocks()
	existsResource.mockResolvedValue(false)
	existsVariable.mockResolvedValue(false)
})

// Writing over a variable retargets every `$var:` reference to it at once, and
// deleting one deletes the resource sharing its path, so ownership is proven
// from the description this module stamps rather than assumed from the path.
describe('upsertSecretVariable', () => {
	it('refuses a variable it did not write', async () => {
		existsVariable.mockResolvedValue(true)
		getVariable.mockResolvedValue({ description: "someone else's key" })

		await expect(upsertSecretVariable(ARGS)).rejects.toThrow('already exists')
		expect(updateVariable).not.toHaveBeenCalled()
		expect(deleteVariable).not.toHaveBeenCalled()
	})

	it('refuses to touch anything while a connection occupies the path', async () => {
		existsResource.mockResolvedValue(true)

		await expect(upsertSecretVariable(ARGS)).rejects.toThrow('A connection already exists')
		expect(existsVariable).not.toHaveBeenCalled()
		expect(updateVariable).not.toHaveBeenCalled()
	})

	it('restates is_secret when replacing its own token', async () => {
		existsVariable.mockResolvedValue(true)
		getVariable.mockResolvedValue({ description: OURS })

		await upsertSecretVariable(ARGS)

		expect(updateVariable).toHaveBeenCalledWith({
			workspace: 'ws',
			path: ARGS.path,
			requestBody: { value: 'token', is_secret: true }
		})
	})

	// An OAuth variable carries the account its refresh runs through, which
	// `EditVariable` cannot change, so it is recreated rather than patched.
	it('recreates its own oauth token instead of patching it', async () => {
		existsVariable.mockResolvedValue(true)
		getVariable.mockResolvedValue({ description: OURS })

		await upsertSecretVariable({ ...ARGS, isOauth: true, account: 7 })

		expect(deleteVariable).toHaveBeenCalled()
		expect(createVariable).toHaveBeenCalledWith({
			workspace: 'ws',
			requestBody: {
				path: ARGS.path,
				value: 'token',
				is_secret: true,
				is_oauth: true,
				account: 7,
				description: OURS
			}
		})
	})
})

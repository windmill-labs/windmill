import { beforeEach, describe, expect, it, vi } from 'vitest'

const { listMock } = vi.hoisted(() => ({ listMock: vi.fn() }))

vi.mock('./shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: { listDucklakes: listMock }
}))

import { getDucklakeTools } from './ducklakeTools'

function createToolCallbacks() {
	return { setToolStatus: vi.fn(), removeToolStatus: vi.fn() }
}

function run(name: string, args: Record<string, unknown> = {}) {
	const tool = getDucklakeTools().find((entry) => entry.def.function.name === name)
	if (!tool) throw new Error(`${name} tool not found`)
	return tool.fn({
		args,
		workspace: 'test-workspace',
		helpers: {},
		toolCallbacks: createToolCallbacks(),
		toolId: `tool-${name}`
	})
}

beforeEach(() => listMock.mockReset())

describe('list_ducklakes', () => {
	it('returns the configured catalog names', async () => {
		listMock.mockResolvedValue(['main', 'analytics'])
		const result = await run('list_ducklakes')
		expect(listMock).toHaveBeenCalledWith({ workspace: 'test-workspace' })
		expect(JSON.parse(result)).toEqual({ ducklakes: ['main', 'analytics'] })
	})

	it('explains the storage prerequisite with role-appropriate steps when none exist', async () => {
		listMock.mockResolvedValue([])
		const result = await run('list_ducklakes')
		expect(result).toContain('No DuckLake catalogs are configured')
		expect(result).toContain('Workspace settings → Object Storage')
		expect(result).toContain('ask a workspace admin')
		// Drafting is not blocked: the message must say the scripts can still be drafted.
		expect(result).toContain('still draft the pipeline scripts')
	})
})

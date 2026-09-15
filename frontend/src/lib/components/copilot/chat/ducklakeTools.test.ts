import { beforeEach, describe, expect, it, vi } from 'vitest'

const { listMock, listMetricsMock } = vi.hoisted(() => ({
	listMock: vi.fn(),
	listMetricsMock: vi.fn()
}))

vi.mock('./shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: { listDucklakes: listMock },
	DataMetricService: { listDataMetrics: listMetricsMock }
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

beforeEach(() => {
	listMock.mockReset()
	listMetricsMock.mockReset()
})

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

describe('list_data_metrics', () => {
	it('forwards the filters and returns the declarations', async () => {
		listMetricsMock.mockResolvedValue({
			metrics: [
				{
					script_path: 'f/analytics/rev',
					table_path: 'main/main.orders',
					kind: 'measure',
					name: 'revenue',
					expr: 'sum(amount)',
					filter: 'not is_test'
				}
			]
		})
		const result = await run('list_data_metrics', {
			table: 'ducklake://main/main.orders',
			path_prefix: 'f/analytics',
			limit: 50
		})
		expect(listMetricsMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			table: 'ducklake://main/main.orders',
			pathPrefix: 'f/analytics',
			perPage: 50
		})
		expect(JSON.parse(result).metrics[0]).toMatchObject({ name: 'revenue', expr: 'sum(amount)' })
	})

	it('never reports an empty result as proof that nothing is declared', async () => {
		listMetricsMock.mockResolvedValue({ metrics: [] })
		const result = await run('list_data_metrics', {})
		// Unreadable declarations are omitted, not flagged, so absence is unprovable.
		expect(result).toContain('does not establish')
		expect(result).toContain('cannot read')
	})

	it('warns that more declarations exist when the page is cut short', async () => {
		listMetricsMock.mockResolvedValue({
			// A cursor only comes back on a full page, never with an empty one.
			metrics: [{ table_path: 't', kind: 'measure', name: 'n', script_path: 's' }],
			next_cursor: { table_path: 't', kind: 'measure', name: 'n', script_path: 's' }
		})
		const result = await run('list_data_metrics', {})
		// Without this the model reads a partial page as "no such measure" and
		// re-derives a number that disagrees with the declared one.
		expect(result).toContain('rather than concluding a measure is undeclared')
	})
})

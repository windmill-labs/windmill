import { beforeEach, describe, expect, it, vi } from 'vitest'

const { listMock, schemaMock, runScriptMock, executeTestRunMock } = vi.hoisted(() => ({
	listMock: vi.fn(),
	schemaMock: vi.fn(),
	runScriptMock: vi.fn(),
	executeTestRunMock: vi.fn()
}))

vi.mock('./shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	}),
	executeTestRun: executeTestRunMock
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		listDataTableTables: listMock,
		getDataTableTableSchema: schemaMock
	}
}))

vi.mock('$lib/components/jobs/utils', () => ({
	runScript: runScriptMock
}))

// exec_datatable_sql now routes through executeTestRun; stub it to exercise the
// jobStarter (so runScript args can be asserted) and then hand a completed job to
// the tool's formatCompletion — which owns the datatable-specific result shaping.
// The detach/tray orchestration itself is covered by shared.test.ts.
function stubExecuteTestRunWithJob(job: { success: boolean; result: unknown }) {
	runScriptMock.mockResolvedValue('job-123')
	executeTestRunMock.mockImplementation(async (config: any) => {
		await config.jobStarter()
		const { llmText, card } = config.formatCompletion(job)
		config.toolCallbacks.setToolStatus(config.toolId, card)
		return llmText
	})
}

import { getDatatableTools } from './datatableTools'

function createToolCallbacks() {
	return {
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	}
}

function getTool(name: string) {
	const tool = getDatatableTools().find((entry) => entry.def.function.name === name)
	if (!tool) {
		throw new Error(`${name} tool not found`)
	}
	return tool
}

function run(name: string, args: Record<string, unknown> = {}) {
	return getTool(name).fn({
		args,
		workspace: 'test-workspace',
		helpers: {},
		toolCallbacks: createToolCallbacks(),
		toolId: `tool-${name}`
	})
}

beforeEach(() => {
	listMock.mockReset()
	schemaMock.mockReset()
	runScriptMock.mockReset()
	executeTestRunMock.mockReset()
})

describe('list_datatables', () => {
	it('aggregates the table count across schemas and returns metadata verbatim', async () => {
		const metadata = [
			{ datatable_name: 'main', schemas: { public: ['users', 'orders'], analytics: ['events'] } },
			{ datatable_name: 'warehouse', schemas: { public: ['facts'] } }
		]
		listMock.mockResolvedValue(metadata)

		const tool = getTool('list_datatables')
		const callbacks = createToolCallbacks()
		const result = await tool.fn({
			args: {},
			workspace: 'test-workspace',
			helpers: {},
			toolCallbacks: callbacks,
			toolId: 'tool-list'
		})

		expect(listMock).toHaveBeenCalledWith({ workspace: 'test-workspace' })
		expect(JSON.parse(result)).toEqual(metadata)
		// 2 + 1 + 1 = 4 tables across 2 datatables
		expect(callbacks.setToolStatus).toHaveBeenCalledWith('tool-list', {
			content: 'Listed 2 datatable(s) with 4 table(s)'
		})
	})

	it('explains that configuring a datatable is a blocking prerequisite when none exist', async () => {
		listMock.mockResolvedValue([])
		const result = await run('list_datatables')
		expect(result).toContain('No datatables are configured in this workspace')
		expect(result).toContain('Workspace settings → Data Tables')
		expect(result).toContain('blocked')
		expect(result).toContain('Do not call exec_datatable_sql')
	})

	it('surfaces backend errors as a readable message', async () => {
		listMock.mockRejectedValue(new Error('boom'))
		const result = await run('list_datatables')
		expect(result).toContain('Error listing datatables: boom')
	})
})

describe('get_datatable_table_schema', () => {
	it('returns the columns for one table', async () => {
		schemaMock.mockResolvedValue({
			datatable_name: 'main',
			schema_name: 'public',
			table_name: 'users',
			columns: { id: 'int4', email: 'text' }
		})

		const result = await run('get_datatable_table_schema', {
			datatable_name: 'main',
			schema_name: 'public',
			table_name: 'users'
		})

		expect(schemaMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			datatableName: 'main',
			schemaName: 'public',
			tableName: 'users'
		})
		expect(JSON.parse(result)).toEqual({
			datatable_name: 'main',
			schema_name: 'public',
			table_name: 'users',
			columns: { id: 'int4', email: 'text' }
		})
	})
})

describe('exec_datatable_sql', () => {
	it('requires confirmation', () => {
		expect(getTool('exec_datatable_sql').requiresConfirmation).toBe(true)
	})

	it('starts the query as a postgresql job scoped to the datatable', async () => {
		stubExecuteTestRunWithJob({ success: true, result: [] })
		await run('exec_datatable_sql', { datatable_name: 'main', sql: 'SELECT * FROM t' })
		expect(runScriptMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			requestBody: {
				language: 'postgresql',
				content: 'SELECT * FROM t',
				args: { database: 'datatable://main' }
			}
		})
	})

	it('returns all rows when the result is at or below the cap', async () => {
		const rows = Array.from({ length: 100 }, (_, i) => ({ id: i }))
		stubExecuteTestRunWithJob({ success: true, result: rows })

		const parsed = JSON.parse(
			await run('exec_datatable_sql', { datatable_name: 'main', sql: 'SELECT * FROM t' })
		)
		expect(parsed.success).toBe(true)
		expect(parsed.rowCount).toBe(100)
		expect(parsed.result).toHaveLength(100)
		expect(parsed.note).toBeUndefined()
	})

	it('truncates results above the cap and reports the full count', async () => {
		const rows = Array.from({ length: 150 }, (_, i) => ({ id: i }))
		stubExecuteTestRunWithJob({ success: true, result: rows })

		const parsed = JSON.parse(
			await run('exec_datatable_sql', { datatable_name: 'main', sql: 'SELECT 1' })
		)
		expect(parsed.rowCount).toBe(150)
		expect(parsed.result).toHaveLength(100)
		expect(parsed.note).toBe('Showing first 100 of 150 rows')
	})

	it('treats a non-array result (e.g. DDL) as zero rows', async () => {
		stubExecuteTestRunWithJob({ success: true, result: undefined })
		const parsed = JSON.parse(
			await run('exec_datatable_sql', {
				datatable_name: 'main',
				sql: 'CREATE TABLE t (id serial primary key)'
			})
		)
		expect(parsed).toEqual({ success: true, rowCount: 0, result: [] })
	})

	it('returns a failure object when the SQL job errors', async () => {
		stubExecuteTestRunWithJob({ success: false, result: { error: { message: 'syntax error' } } })
		const parsed = JSON.parse(
			await run('exec_datatable_sql', { datatable_name: 'main', sql: 'SLECT' })
		)
		expect(parsed).toEqual({ success: false, error: 'syntax error' })
	})

	it('turns the backend "datatable not found" error into an actionable, blocking message', async () => {
		stubExecuteTestRunWithJob({
			success: false,
			result: { error: { message: 'Internal: datatable main not found @workspaces.rs:565:20' } }
		})
		const parsed = JSON.parse(
			await run('exec_datatable_sql', { datatable_name: 'main', sql: 'CREATE TABLE t (id int)' })
		)
		expect(parsed.success).toBe(false)
		expect(parsed.error).toContain('not configured in this workspace')
		expect(parsed.error).toContain('Workspace settings → Data Tables')
		expect(parsed.error).toContain('do not retry')
		// The raw internal error is not surfaced to the model.
		expect(parsed.error).not.toContain('workspaces.rs')
	})
})

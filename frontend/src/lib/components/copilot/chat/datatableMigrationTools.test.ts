import { beforeEach, describe, expect, it, vi } from 'vitest'

const { statusMock, createMock, runMock, deleteMock } = vi.hoisted(() => ({
	statusMock: vi.fn(),
	createMock: vi.fn(),
	runMock: vi.fn(),
	deleteMock: vi.fn()
}))

vi.mock('./shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		getDatatableMigrationsStatus: statusMock,
		createDatatableMigration: createMock,
		runDatatableMigrations: runMock,
		deleteDatatableMigration: deleteMock
	}
}))

import { getDatatableMigrationTools } from './datatableMigrationTools'

function create(args: Record<string, unknown>) {
	const tool = getDatatableMigrationTools().find(
		(entry) => entry.def.function.name === 'create_datatable_migration'
	)!
	return tool.fn({
		args: {
			datatable_name: 'main',
			name: 'add_refunds',
			code_up: 'CREATE TABLE refunds ();',
			...args
		},
		workspace: 'test-workspace',
		helpers: {},
		toolCallbacks: { setToolStatus: vi.fn(), removeToolStatus: vi.fn() } as any,
		toolId: 'tool-1'
	})
}

const applied = { timestamp: 10, name: 'init', code_up: '', status: 'ran' }
const created = { timestamp: 30, name: 'add_refunds', code_up: '', status: 'not_run' }

beforeEach(() => {
	for (const mock of [statusMock, createMock, runMock, deleteMock]) mock.mockReset()
	createMock.mockResolvedValue({ timestamp: 30 })
	statusMock.mockResolvedValue({ enabled: true, migrations: [applied, created] })
	runMock.mockResolvedValue({ applied: [{ version: 30, name: 'add_refunds' }] })
})

describe('create_datatable_migration', () => {
	it('applies only the migration it created', async () => {
		const result = await create({})
		expect(runMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			datatableName: 'main',
			only: 30
		})
		expect(deleteMock).not.toHaveBeenCalled()
		expect(result).toContain('Created and applied migration 30 (add_refunds)')
	})

	it('leaves it pending when an earlier migration is not applied', async () => {
		statusMock.mockResolvedValue({
			enabled: true,
			migrations: [
				applied,
				{ timestamp: 20, name: 'earlier', code_up: '', status: 'not_run' },
				created
			]
		})
		const result = await create({})
		expect(runMock).not.toHaveBeenCalled()
		expect(deleteMock).not.toHaveBeenCalled()
		expect(result).toContain('20 (earlier)')
	})

	it('discards a migration that fails to apply', async () => {
		runMock.mockRejectedValue({ body: 'relation "refunds" already exists' })
		const result = await create({})
		expect(deleteMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			datatableName: 'main',
			timestamp: 30
		})
		expect(result).toContain('relation "refunds" already exists')
		expect(result).toContain('discarded')
	})

	it('declares the role above the SQL of both directions', async () => {
		await create({ role: 'app_rw', code_down: 'DROP TABLE refunds;', apply: false })
		expect(createMock.mock.calls[0][0].requestBody).toEqual({
			name: 'add_refunds',
			code_up: '-- role app_rw\nCREATE TABLE refunds ();',
			code_down: '-- role app_rw\nDROP TABLE refunds;'
		})
		expect(runMock).not.toHaveBeenCalled()
	})
})

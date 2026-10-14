import { beforeEach, describe, expect, it, vi } from 'vitest'

const { statusMock, createMock, upsertMock, runMock } = vi.hoisted(() => ({
	upsertMock: vi.fn(),
	statusMock: vi.fn(),
	createMock: vi.fn(),
	runMock: vi.fn()
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
		upsertDatatableMigration: upsertMock,
		runDatatableMigrations: runMock
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
			code_up: 'BEGIN;\nCREATE TABLE refunds ();\nEND;',
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
	for (const mock of [statusMock, createMock, upsertMock, runMock]) mock.mockReset()
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
		expect(result).toContain('Applied migration 30 (add_refunds)')
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
		expect(result).toContain('20 (earlier)')
	})

	it('keeps a migration that fails to apply and rewrites it on request', async () => {
		runMock.mockRejectedValueOnce({ body: 'relation "refunds" already exists' })
		const failed = await create({})
		expect(failed).toContain('relation "refunds" already exists')
		expect(failed).toContain('with version 30')

		statusMock.mockResolvedValue({
			enabled: true,
			migrations: [applied, { ...created, code_down: 'DROP TABLE refunds;' }]
		})
		await create({ version: 30, code_up: 'BEGIN;\nCREATE TABLE refunds2 ();\nEND;' })
		expect(createMock).toHaveBeenCalledTimes(1)
		expect(upsertMock).toHaveBeenCalledWith({
			workspace: 'test-workspace',
			datatableName: 'main',
			requestBody: {
				timestamp: 30,
				code_down: 'DROP TABLE refunds;',
				name: 'add_refunds',
				code_up: 'BEGIN;\nCREATE TABLE refunds2 ();\nEND;'
			}
		})
		expect(runMock).toHaveBeenLastCalledWith({
			workspace: 'test-workspace',
			datatableName: 'main',
			only: 30
		})
	})

	it('refuses to rewrite a version that is not pending', async () => {
		const result = await create({ version: 10 })
		expect(upsertMock).not.toHaveBeenCalled()
		expect(runMock).not.toHaveBeenCalled()
		expect(result).toContain('cannot be rewritten')
	})

	it('does not call unreadable earlier migrations pending', async () => {
		statusMock.mockResolvedValue({
			enabled: true,
			migrations: [
				{ ...applied, status: 'unknown' },
				{ ...created, status: 'unknown' }
			],
			error: 'connection refused'
		})
		const result = await create({})
		expect(runMock).not.toHaveBeenCalled()
		expect(result).toContain('connection refused')
		expect(result).not.toContain('10 (init)')
	})

	it('declares the role above the SQL of both directions', async () => {
		await create({ role: 'app_rw', code_down: 'DROP TABLE refunds;', apply: false })
		expect(createMock.mock.calls[0][0].requestBody).toEqual({
			name: 'add_refunds',
			code_up: '-- role app_rw\nBEGIN;\nCREATE TABLE refunds ();\nEND;',
			code_down: '-- role app_rw\nDROP TABLE refunds;'
		})
		expect(runMock).not.toHaveBeenCalled()
	})
})

import { describe, expect, it, vi } from 'vitest'
import { getAppTools, type AppAIChatHelpers, type BackendRunnable, type LintResult } from './core'

vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: {
			name,
			description,
			parameters: {}
		}
	}),
	createSearchWorkspaceTool: () => ({
		def: {
			type: 'function',
			function: {
				name: 'search_workspace',
				description: 'search workspace',
				parameters: {}
			}
		},
		fn: async () => ''
	}),
	createGetRunnableDetailsTool: () => ({
		def: {
			type: 'function',
			function: {
				name: 'get_runnable_details',
				description: 'get runnable details',
				parameters: {}
			}
		},
		fn: async () => ''
	}),
	createSearchHubScriptsTool: () => ({
		def: {
			type: 'function',
			function: {
				name: 'search_hub_scripts',
				description: 'search hub scripts',
				parameters: {}
			}
		},
		fn: async () => ''
	})
}))

vi.mock('../AIChatManager.svelte', () => ({
	aiChatManager: {
		datatableCreationPolicy: {
			enabled: false,
			datatable: undefined,
			schema: undefined
		}
	}
}))

vi.mock('$system_prompts', () => ({
	getDatatableSdkReference: () => 'Datatable SDK reference'
}))

const EMPTY_LINT_RESULT: LintResult = {
	errorCount: 0,
	warningCount: 0,
	errors: {
		frontend: {},
		backend: {}
	},
	warnings: {
		frontend: {},
		backend: {}
	}
}

function createHelpers(overrides: Partial<AppAIChatHelpers> = {}): AppAIChatHelpers {
	return {
		listFrontendFiles: () => [],
		getFrontendFile: () => undefined,
		getFrontendFiles: () => ({}),
		setFrontendFile: () => EMPTY_LINT_RESULT,
		deleteFrontendFile: () => undefined,
		listBackendRunnables: () => [],
		getBackendRunnable: () => undefined,
		getBackendRunnables: () => ({}),
		setBackendRunnable: async () => EMPTY_LINT_RESULT,
		deleteBackendRunnable: () => undefined,
		getFiles: () => ({ frontend: {}, backend: {} }),
		getSelectedContext: () => ({ type: 'none' }),
		snapshot: () => 1,
		revertToSnapshot: () => undefined,
		lint: () => EMPTY_LINT_RESULT,
		getDatatables: async () => [],
		getAvailableDatatableNames: () => [],
		execDatatableSql: async () => ({ success: true }),
		addTableToWhitelist: () => undefined,
		...overrides
	}
}

function createToolCallbacks() {
	return {
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	}
}

function getPatchFileTool() {
	const tool = getAppTools().find((entry) => entry.def.function.name === 'patch_file')
	if (!tool) {
		throw new Error('patch_file tool not found')
	}
	return tool
}

describe('app patch_file tool', () => {
	it('patches frontend files with an exact replacement', async () => {
		const setFrontendFile = vi.fn(() => EMPTY_LINT_RESULT)
		const tool = getPatchFileTool()

		const result = await tool.fn({
			args: {
				path: '/index.tsx',
				old_string: 'Hello recipe book',
				new_string: 'Hello cookbook'
			},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getFrontendFile: () => 'export const title = "Hello recipe book"\n',
				setFrontendFile
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-1'
		})

		expect(setFrontendFile).toHaveBeenCalledWith('/index.tsx', 'export const title = "Hello cookbook"\n')
		expect(result).toContain("Patched '/index.tsx' successfully.")
	})

	it('rejects ambiguous matches unless replace_all is set', async () => {
		const tool = getPatchFileTool()

		await expect(
			tool.fn({
				args: {
					path: '/index.tsx',
					old_string: 'recipe',
					new_string: 'meal'
				},
				workspace: 'test-workspace',
				helpers: createHelpers({
					getFrontendFile: () => 'recipe\nrecipe\n'
				}),
				toolCallbacks: createToolCallbacks(),
				toolId: 'tool-2'
			})
		).rejects.toThrow('old_string matched 2 locations')
	})

	it('patches inline backend runnables through backend/<key>/main.ts paths', async () => {
		const runnable: BackendRunnable = {
			name: 'Delete recipe',
			type: 'inline',
			inlineScript: {
				language: 'bun',
				content: 'export async function main() {\n\treturn "recipe"\n}\n'
			}
		}
		const setBackendRunnable = vi.fn(async () => EMPTY_LINT_RESULT)
		const tool = getPatchFileTool()

		const result = await tool.fn({
			args: {
				path: 'backend/deleteRecipe/main.ts',
				old_string: '"recipe"',
				new_string: '"meal"'
			},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getBackendRunnable: () => runnable,
				setBackendRunnable
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-3'
		})

		expect(setBackendRunnable).toHaveBeenCalledWith(
			'deleteRecipe',
			expect.objectContaining({
				inlineScript: expect.objectContaining({
					content: 'export async function main() {\n\treturn "meal"\n}\n'
				})
			})
		)
		expect(result).toContain("Patched 'backend/deleteRecipe/main.ts' successfully.")
	})

	it('rejects edits to generated wmill types', async () => {
		const tool = getPatchFileTool()

		await expect(
			tool.fn({
				args: {
					path: '/wmill.d.ts',
					old_string: 'backend',
					new_string: 'backendAsync'
				},
				workspace: 'test-workspace',
				helpers: createHelpers({
					getFrontendFile: () => 'generated'
				}),
				toolCallbacks: createToolCallbacks(),
				toolId: 'tool-4'
			})
		).rejects.toThrow('generated automatically')
	})
})

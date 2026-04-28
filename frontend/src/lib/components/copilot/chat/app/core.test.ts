import { describe, expect, it, vi } from 'vitest'
import {
	getAppTools,
	prepareAppUserMessage,
	type AppAIChatHelpers,
	type BackendRunnable,
	type LintResult,
	type SelectedContext
} from './core'
import type { ContextElement } from '../context'

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
		getSelectedContext: () => ({}),
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

function getTool(name: string) {
	const tool = getAppTools().find((entry) => entry.def.function.name === name)
	if (!tool) {
		throw new Error(`${name} tool not found`)
	}
	return tool
}

function getListFilesTool() {
	return getTool('list_files')
}

function getPatchFileTool() {
	return getTool('patch_file')
}

describe('app list_files tool', () => {
	it('returns lightweight metadata without file or runnable contents', async () => {
		const tool = getListFilesTool()

		const result = await tool.fn({
			args: {},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getFiles: () => ({
					frontend: {
						'/index.tsx': 'const secretFrontendContent = true',
						'/styles.css': '.secret-class { color: red; }'
					},
					backend: {
						loadUsers: {
							name: 'Load users',
							type: 'inline',
							staticInputs: { admin: true },
							inlineScript: {
								language: 'bun',
								content: 'export async function main() { return "secretBackendContent" }'
							}
						},
						workspaceFlow: {
							name: 'Workspace flow',
							type: 'flow',
							path: 'f/flows/workspace_flow'
						}
					}
				})
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-list-files'
		})

		const parsed = JSON.parse(result)
		expect(parsed).toEqual({
			frontend: [
				{ path: '/index.tsx', size: 34, kind: 'tsx' },
				{ path: '/styles.css', size: 29, kind: 'css' }
			],
			backend: [
				{
					key: 'loadUsers',
					name: 'Load users',
					type: 'inline',
					language: 'bun',
					contentSize: 62,
					staticInputKeys: ['admin']
				},
				{
					key: 'workspaceFlow',
					name: 'Workspace flow',
					type: 'flow',
					path: 'f/flows/workspace_flow'
				}
			]
		})
		expect(result).not.toContain('secretFrontendContent')
		expect(result).not.toContain('secretBackendContent')
	})
})

describe('app datatable tools', () => {
	it('lists datatable metadata without column definitions', async () => {
		const tool = getTool('list_datatables')

		const result = await tool.fn({
			args: {},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getDatatables: async () => [
					{
						datatable_name: 'main',
						schemas: {
							public: {
								users: { id: 'int4', email: 'text' },
								orders: { id: 'int4', total: 'numeric' }
							},
							analytics: {
								events: { id: 'int4', payload: 'jsonb' }
							}
						}
					}
				]
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-list-datatables'
		})

		const parsed = JSON.parse(result)
		expect(parsed).toEqual([
			{
				datatable_name: 'main',
				schemas: {
					public: ['users', 'orders'],
					analytics: ['events']
				},
				tableCount: 3
			}
		])
		expect(result).not.toContain('email')
		expect(result).not.toContain('jsonb')
	})

	it('gets one datatable table schema', async () => {
		const tool = getTool('get_datatable_table_schema')

		const result = await tool.fn({
			args: {
				datatable_name: 'main',
				schema_name: 'public',
				table_name: 'users'
			},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getDatatables: async () => [
					{
						datatable_name: 'main',
						schemas: {
							public: {
								users: { id: 'int4', email: 'text' },
								orders: { id: 'int4', total: 'numeric' }
							}
						}
					}
				]
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-get-table-schema'
		})

		const parsed = JSON.parse(result)
		expect(parsed).toEqual({
			datatable_name: 'main',
			schema_name: 'public',
			table_name: 'users',
			columns: { id: 'int4', email: 'text' }
		})
		expect(result).not.toContain('total')
	})
})

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

	it('treats leading /backend paths as frontend files', async () => {
		const setFrontendFile = vi.fn(() => EMPTY_LINT_RESULT)
		const tool = getPatchFileTool()

		const result = await tool.fn({
			args: {
				path: '/backend/deleteRecipe/main.ts',
				old_string: 'frontend route',
				new_string: 'frontend page'
			},
			workspace: 'test-workspace',
			helpers: createHelpers({
				getFrontendFile: () => 'export const title = "frontend route"\n',
				setFrontendFile
			}),
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-leading-backend-path'
		})

		expect(setFrontendFile).toHaveBeenCalledWith(
			'/backend/deleteRecipe/main.ts',
			'export const title = "frontend page"\n'
		)
		expect(result).toContain("Patched '/backend/deleteRecipe/main.ts' successfully.")
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

describe('prepareAppUserMessage app context', () => {
	it('still serializes inspector and code selections', () => {
		const selectedContext: SelectedContext = {
			type: 'frontend',
			frontendPath: '/index.tsx',
			inspectorElement: {
				path: 'body > button.primary',
				tagName: 'button',
				id: 'save',
				className: 'primary action',
				rect: { top: 10, left: 20, width: 120, height: 40 },
				html: '<button id="save" class="primary action">Save</button>',
				textContent: 'Save',
				styles: {}
			},
			codeSelection: {
				type: 'app_code_selection',
				source: '/index.tsx',
				sourceType: 'frontend',
				title: '/index.tsx:3-4',
				content: 'const selectedCode = true',
				startLine: 3,
				endLine: 4,
				startColumn: 1,
				endColumn: 25
			}
		} as unknown as SelectedContext

		const message = prepareAppUserMessage('Change this selected area', selectedContext)

		const content = message.content as string
		expect(content).toContain('The user has selected an element in the app preview')
		expect(content).toContain('body > button.primary')
		expect(content).toContain('### CODE SELECTION:')
		expect(content).toContain('const selectedCode = true')
	})

	it('serializes explicit mentions with lightweight file context', () => {
		const additionalContext: ContextElement[] = [
			{
				type: 'app_frontend_file',
				path: '/index.tsx',
				title: '/index.tsx',
				content: 'const fullFrontendContent = true'
			},
			{
				type: 'app_backend_runnable',
				key: 'loadUsers',
				title: 'loadUsers',
				runnable: {
					name: 'Load users',
					type: 'inline',
					staticInputs: { admin: true },
					inlineScript: {
						language: 'bun',
						content: 'export async function main() { return "secret" }'
					}
				}
			},
			{
				type: 'app_datatable',
				datatableName: 'main',
				schemaName: 'public',
				tableName: 'users',
				title: 'main/users',
				columns: {
					id: 'int4',
					email: 'text'
				}
			}
		]

		const message = prepareAppUserMessage('Wire these together', undefined, additionalContext)

		const content = message.content as string
		expect(content).toContain('- Frontend file: /index.tsx')
		expect(content).toContain('- Backend runnable: loadUsers')
		expect(content).not.toContain('fullFrontendContent')
		expect(content).not.toContain('export async function main')
		expect(content).not.toContain('Static inputs')
		expect(content).not.toContain('Load users')
		expect(content).toContain('**Table: main/users**')
		expect(content).toContain('"id": "int4"')
		expect(content).toContain('"email": "text"')
	})
})

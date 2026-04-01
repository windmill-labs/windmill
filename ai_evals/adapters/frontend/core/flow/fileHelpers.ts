import { mkdir, rm, writeFile } from 'fs/promises'
import { dirname, join } from 'path'
import type { FlowModule, InputTransform } from '../../../../../frontend/src/lib/gen'
import type { ExtendedOpenFlow } from '../../../../../frontend/src/lib/components/flows/types'
import type { FlowAIChatHelpers } from '../../../../../frontend/src/lib/components/copilot/chat/flow/core'
import type { ScriptLintResult } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import { findModuleById } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import {
	inlineScriptStore,
	restoreInlineScriptReferences
} from '../../../../../frontend/src/lib/components/copilot/chat/flow/inlineScriptsUtils'

const EMPTY_SCRIPT_LINT_RESULT: ScriptLintResult = {
	errorCount: 0,
	warningCount: 0,
	errors: [],
	warnings: []
}

export async function createFlowFileHelpers(
	initialModules: FlowModule[] = [],
	initialSchema?: Record<string, any>,
	workspaceRoot?: string
): Promise<{
	helpers: FlowAIChatHelpers
	getFlow: () => ExtendedOpenFlow
	getModules: () => FlowModule[]
	cleanup: () => Promise<void>
	workspaceDir: string | null
}> {
	let flow: ExtendedOpenFlow = {
		value: { modules: structuredClone(initialModules) },
		summary: '',
		schema: initialSchema ?? {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: {},
			required: [],
			type: 'object'
		}
	}

	const flowFilePath = workspaceRoot ? join(workspaceRoot, 'flow.json') : null

	async function persistFlow(): Promise<void> {
		if (!flowFilePath) {
			return
		}
		await mkdir(dirname(flowFilePath), { recursive: true })
		await writeFile(flowFilePath, JSON.stringify(flow, null, 2) + '\n', 'utf8')
	}

	await persistFlow()

	const helpers: FlowAIChatHelpers = {
		getFlowAndSelectedId: () => ({ flow, selectedId: '' }),
		getModules: (id?: string) => {
			if (!id) return flow.value.modules
			const module = findModuleById(flow.value.modules, id)
			return module ? [module] : []
		},
		setSnapshot: () => {},
		revertToSnapshot: () => {},
		setCode: async (id: string, code: string) => {
			const module = findModuleById(flow.value.modules, id)
			if (module && module.value.type === 'rawscript') {
				module.value.content = code
			}
			inlineScriptStore.set(id, code)
			await persistFlow()
		},
		setFlowJson: async (
			modules: FlowModule[] | undefined,
			schema: Record<string, any> | undefined
		) => {
			if (modules) {
				flow.value.modules = restoreInlineScriptReferences(modules)
			}
			if (schema !== undefined) {
				flow.schema = schema
			}
			await persistFlow()
		},
		getFlowInputsSchema: async () => flow.schema ?? {},
		updateExprsToSet: (_id: string, _inputTransforms: Record<string, InputTransform>) => {},
		acceptAllModuleActions: () => {},
		rejectAllModuleActions: () => {},
		hasPendingChanges: () => false,
		selectStep: (_id: string) => {},
		testFlow: async (args?: Record<string, any>) => {
			if (workspaceRoot) {
				const runPath = join(workspaceRoot, 'test-run.json')
				await writeFile(
					runPath,
					JSON.stringify(
						{
							requestedArgs: args ?? {},
							modules: flow.value.modules.map((module) => module.id)
						},
						null,
						2
					) + '\n',
					'utf8'
				)
			}
			return `mock-job-id-${Date.now()}`
		},
		getLintErrors: async () => EMPTY_SCRIPT_LINT_RESULT
	}

	return {
		helpers,
		getFlow: () => flow,
		getModules: () => flow.value.modules,
		cleanup: async () => {
			if (workspaceRoot) {
				await rm(workspaceRoot, { recursive: true, force: true })
			}
		},
		workspaceDir: workspaceRoot ?? null
	}
}

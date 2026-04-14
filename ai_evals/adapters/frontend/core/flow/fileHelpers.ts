import { mkdir, rm, writeFile } from 'fs/promises'
import { dirname, join } from 'path'
import type { FlowModule, InputTransform } from '../../../../../frontend/src/lib/gen'
import type { ExtendedOpenFlow } from '../../../../../frontend/src/lib/components/flows/types'
import type { FlowAIChatHelpers } from '../../../../../frontend/src/lib/components/copilot/chat/flow/core'
import type { ScriptLintResult } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import {
	findModuleById,
	SPECIAL_MODULE_IDS
} from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import {
	createInlineScriptSession
} from '../../../../../frontend/src/lib/components/copilot/chat/flow/inlineScriptsUtils'
import {
	registerBenchmarkWorkspace,
	registerBenchmarkWorkspaceRunnables,
	unregisterBenchmarkWorkspaceRunnables,
	createBenchmarkCompletedJob,
	type BenchmarkWorkspaceFlow,
	type BenchmarkWorkspaceScript
} from '../../mockBackend'

const EMPTY_SCRIPT_LINT_RESULT: ScriptLintResult = {
	errorCount: 0,
	warningCount: 0,
	errors: [],
	warnings: []
}

export interface FlowWorkspaceFixtures {
	scripts?: BenchmarkWorkspaceScript[]
	flows?: BenchmarkWorkspaceFlow[]
}

export async function createFlowFileHelpers(
	initialModules: FlowModule[] = [],
	initialSchema?: Record<string, any>,
	initialPreprocessorModule?: FlowModule,
	initialFailureModule?: FlowModule,
	workspaceRoot?: string,
	workspaceFixtures?: FlowWorkspaceFixtures
): Promise<{
	helpers: FlowAIChatHelpers
	getFlow: () => ExtendedOpenFlow
	getModules: () => FlowModule[]
	cleanup: () => Promise<void>
	workspaceDir: string | null
}> {
	let flow: ExtendedOpenFlow = {
		value: {
			modules: structuredClone(initialModules),
			preprocessor_module: structuredClone(initialPreprocessorModule),
			failure_module: structuredClone(initialFailureModule)
		},
		summary: '',
		schema: initialSchema ?? {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: {},
			required: [],
			type: 'object'
		}
	}
	const inlineScriptSession = createInlineScriptSession()

	const flowFilePath = workspaceRoot ? join(workspaceRoot, 'flow.json') : null

	async function persistFlow(): Promise<void> {
		if (!flowFilePath) {
			return
		}
		await mkdir(dirname(flowFilePath), { recursive: true })
		await writeFile(flowFilePath, JSON.stringify(flow, null, 2) + '\n', 'utf8')
	}

	await persistFlow()

	if (workspaceRoot) {
		registerBenchmarkWorkspace(workspaceRoot)
		if (workspaceFixtures) {
			registerBenchmarkWorkspaceRunnables(workspaceRoot, workspaceFixtures)
		}
	}

	const helpers: FlowAIChatHelpers = {
		getFlowAndSelectedId: () => ({ flow, selectedId: '' }),
		getModules: (id?: string) => {
			if (!id) return flow.value.modules
			if (id === SPECIAL_MODULE_IDS.PREPROCESSOR && flow.value.preprocessor_module) {
				return [flow.value.preprocessor_module]
			}
			if (id === SPECIAL_MODULE_IDS.FAILURE && flow.value.failure_module) {
				return [flow.value.failure_module]
			}
			const module = findModuleById(flow.value.modules, id)
			return module ? [module] : []
		},
		inlineScriptSession,
		setSnapshot: () => {},
		revertToSnapshot: () => {},
		setCode: async (id: string, code: string) => {
			const module =
				id === SPECIAL_MODULE_IDS.PREPROCESSOR
					? flow.value.preprocessor_module
					: id === SPECIAL_MODULE_IDS.FAILURE
						? flow.value.failure_module
						: findModuleById(flow.value.modules, id)
			if (module && module.value.type === 'rawscript') {
				module.value.content = code
			}
			inlineScriptSession.set(id, code)
			await persistFlow()
		},
		setFlowJson: async (
			modules: FlowModule[] | undefined,
			schema: Record<string, any> | undefined,
			preprocessorModule: FlowModule | null | undefined,
			failureModule: FlowModule | null | undefined
		) => {
			if (modules !== undefined) {
				flow.value.modules = inlineScriptSession.restoreInlineScriptReferences(modules)
				const unresolvedRefs = inlineScriptSession.findUnresolvedInlineScriptRefs(flow.value.modules)
				if (unresolvedRefs.length > 0) {
					throw new Error(
						`Unresolved inline script references: ${unresolvedRefs.join(', ')}`
					)
				}
			}
			if (schema !== undefined) {
				flow.schema = schema
			}

			const restoreSpecialModule = (module: FlowModule) => {
				const [restoredModule] = inlineScriptSession.restoreInlineScriptReferences([module])
				const unresolvedRefs =
					inlineScriptSession.findUnresolvedInlineScriptRefs([restoredModule])
				if (unresolvedRefs.length > 0) {
					throw new Error(
						`Unresolved inline script references: ${unresolvedRefs.join(', ')}`
					)
				}
				return restoredModule
			}

			if (preprocessorModule !== undefined) {
				flow.value.preprocessor_module =
					preprocessorModule === null ? undefined : restoreSpecialModule(preprocessorModule)
			}

			if (failureModule !== undefined) {
				flow.value.failure_module =
					failureModule === null ? undefined : restoreSpecialModule(failureModule)
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
							modules: flow.value.modules.map((module) => module.id),
							preprocessor_module: flow.value.preprocessor_module?.id ?? null,
							failure_module: flow.value.failure_module?.id ?? null
						},
						null,
						2
					) + '\n',
					'utf8'
				)
			}
			return createBenchmarkCompletedJob({
				workspace: workspaceRoot ?? 'benchmark',
				jobKind: 'flowpreview',
				result: {
					requestedArgs: args ?? {},
					modules: flow.value.modules.map((module) => module.id),
					preprocessor_module: flow.value.preprocessor_module?.id ?? null,
					failure_module: flow.value.failure_module?.id ?? null,
					mocked: true
				},
				logs: 'Mock benchmark flow test run completed successfully.'
			})
		},
		getLintErrors: async () => EMPTY_SCRIPT_LINT_RESULT
	}

	return {
		helpers,
		getFlow: () => flow,
		getModules: () => flow.value.modules,
		cleanup: async () => {
			if (workspaceRoot) {
				unregisterBenchmarkWorkspaceRunnables(workspaceRoot)
			}
			if (workspaceRoot) {
				await rm(workspaceRoot, { recursive: true, force: true })
			}
		},
		workspaceDir: workspaceRoot ?? null
	}
}

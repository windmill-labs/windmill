import type { Schema } from '$lib/common'
import {
	ScriptService,
	type FlowModule,
	type PathFlow,
	type PathScript,
	type RawScript,
	type OpenFlow,
	type Script
} from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import { getScriptByPath } from '$lib/scripts'
import { get, type Writable } from 'svelte/store'
import type { FlowModuleState, FlowState } from './flowState'
import { emptyFlowModuleState } from './utils'
import { NEVER_TESTED_THIS_FAR } from './models'
import { loadSchemaFromModule } from './flowInfers'
import { nextId } from './flowModuleNextId'
import { findNextAvailablePath } from '$lib/path'
import type { ExtendedOpenFlow } from './types'
import { emptySchema } from '$lib/utils'

export async function loadFlowModuleState(flowModule: FlowModule): Promise<FlowModuleState> {
	try {
		const { input_transforms, schema } = await loadSchemaFromModule(flowModule)

		if (
			flowModule.value.type == 'script' ||
			flowModule.value.type == 'rawscript' ||
			flowModule.value.type == 'flow'
		) {
			flowModule.value.input_transforms = input_transforms
		}

		return {
			schema,
			previewResult: NEVER_TESTED_THIS_FAR
		}
	} catch (e) {
		console.debug(e)
		return emptyFlowModuleState()
	}
}

export async function pickScript(
	path: string,
	summary: string,
	id: string,
	hash?: string,
	kind?: string
): Promise<[FlowModule & { value: PathScript }, FlowModuleState]> {
	const flowModule: FlowModule & { value: PathScript } = {
		id,
		value: { type: 'script', path, hash, input_transforms: {}, is_trigger: kind === 'trigger' },
		summary
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function pickFlow(
	path: string,
	summary: string,
	id: string
): Promise<[FlowModule & { value: PathFlow }, FlowModuleState]> {
	const flowModule: FlowModule & { value: PathFlow } = {
		id,
		value: { type: 'flow', path, input_transforms: {} },
		summary
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createInlineScriptModule(
	language: RawScript['language'],
	kind: Script['kind'],
	subkind: 'pgsql' | 'flow' | undefined,
	id: string,
	summary?: string
): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id,
		summary,
		value: {
			type: 'rawscript',
			content: code,
			language,
			input_transforms: {},
			...(kind === 'trigger' ? { is_trigger: true } : {})
		}
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createLoop(
	id: string,
	enabledAi: boolean
): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id,
		value: {
			type: 'forloopflow',
			modules: [],
			iterator: { type: 'javascript', expr: enabledAi ? '' : "['dynamic or static array']" },
			skip_failures: true
		}
	}

	const flowModuleState = await loadFlowModuleState(loopFlowModule)
	return [loopFlowModule, flowModuleState]
}

export async function createWhileLoop(id: string): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id,
		value: {
			type: 'whileloopflow',
			modules: [],
			skip_failures: false
		}
	}

	const flowModuleState = await loadFlowModuleState(loopFlowModule)
	return [loopFlowModule, flowModuleState]
}

export async function createBranches(id: string): Promise<[FlowModule, FlowModuleState]> {
	const branchesFlowModules: FlowModule = {
		id,
		value: {
			type: 'branchone',
			branches: [],
			default: []
		},
		summary: ''
	}

	const flowModuleState = await loadFlowModuleState(branchesFlowModules)

	return [branchesFlowModules, flowModuleState]
}

export async function createBranchAll(id: string): Promise<[FlowModule, FlowModuleState]> {
	const branchesFlowModules: FlowModule = {
		id,
		value: {
			type: 'branchall',
			branches: [{ modules: [] }],
			parallel: true
		},
		summary: ''
	}

	const flowModuleState = await loadFlowModuleState(branchesFlowModules)

	return [branchesFlowModules, flowModuleState]
}

export async function createFlow(id: string): Promise<[FlowModule, FlowModuleState]> {
	const flowFlowModules: FlowModule = {
		id,
		value: {
			type: 'flow',
			path: '',
			input_transforms: {}
		},
		summary: ''
	}

	const flowModuleState = await loadFlowModuleState(flowFlowModules)

	return [flowFlowModules, flowModuleState]
}

export async function fork(
	flowModule: FlowModule
): Promise<[FlowModule & { value: RawScript }, FlowModuleState]> {
	if (flowModule.value.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
	const forkedFlowModule = await createInlineScriptModuleFromPath(
		flowModule.value.path ?? '',
		flowModule.id
	)
	const flowModuleState = await loadFlowModuleState(forkedFlowModule)
	return [forkedFlowModule, flowModuleState]
}

async function createInlineScriptModuleFromPath(
	path: string,
	id: string
): Promise<FlowModule & { value: RawScript }> {
	const { content, language } = await getScriptByPath(path)

	return {
		id,
		value: {
			type: 'rawscript',
			language: language as RawScript['language'],
			content: content,
			path,
			input_transforms: {}
		}
	}
}

export function emptyModule(flowState: FlowState, fullFlow: OpenFlow, flow?: boolean): FlowModule {
	return {
		id: nextId(flowState, fullFlow),
		value: { type: 'identity', flow }
	}
}

export async function createScriptFromInlineScript(
	flowModule: FlowModule,
	suffix: string,
	schema: Schema | undefined,
	flowPath: string
): Promise<[FlowModule & { value: PathScript }, FlowModuleState]> {
	const user = get(userStore)

	if (flowModule.value.type != 'rawscript') {
		throw new Error("Can't create script from non-inline script")
	}

	const originalScriptPath = flowModule.value.path
	const wasForked = Boolean(originalScriptPath)

	if (wasForked && originalScriptPath) {
		const [_first, _second, ...others] = originalScriptPath.split('/')
		suffix = others.join('/')
	}

	const path = `${flowPath}/${suffix}`
	const forkedDescription = wasForked ? `as a fork of ${originalScriptPath}` : ''
	const description = `This script was edited in place of flow ${flowPath} ${forkedDescription} by ${user?.username}.`

	const availablePath = await findNextAvailablePath(path)

	const hash = await ScriptService.createScript({
		workspace: get(workspaceStore)!,
		requestBody: {
			path: availablePath,
			summary: flowModule.summary ?? '',
			description,
			content: flowModule.value.content,
			parent_hash: undefined,
			schema,
			is_template: false,
			language: flowModule.value.language,
			kind: flowModule.value.is_trigger ? 'trigger' : 'script'
		}
	})

	return pickScript(availablePath, flowModule.summary ?? '', flowModule.id, hash)
}

export function deleteFlowStateById(id: string, flowStateStore: Writable<FlowState>) {
	flowStateStore.update((fss) => {
		delete fss[id]
		return fss
	})
}

export function sliceModules(
	modules: FlowModule[],
	upTo: number,
	idOrders: string[]
): FlowModule[] {
	return modules
		.filter((x) => idOrders.indexOf(x.id) <= upTo)
		.map((m) => {
			if (idOrders.indexOf(m.id) == upTo) {
				return m
			}
			if (m.value.type === 'forloopflow') {
				m.value.modules = sliceModules(m.value.modules, upTo, idOrders)
			} else if (m.value.type === 'branchone') {
				m.value.branches = m.value.branches.map((b) => {
					b.modules = sliceModules(b.modules, upTo, idOrders)
					return b
				})
				m.value.default = sliceModules(m.value.default, upTo, idOrders)
			} else if (m.value.type === 'branchall') {
				m.value.branches = m.value.branches.map((b) => {
					b.modules = sliceModules(b.modules, upTo, idOrders)
					return b
				})
			}
			return m
		})
}

export async function insertNewPreprocessorModule(
	flowStore: Writable<ExtendedOpenFlow>,
	flowStateStore: Writable<FlowState>,
	inlineScript?: {
		language: RawScript['language']
	},
	wsScript?: { path: string; summary: string; hash: string | undefined }
) {
	let module: FlowModule = {
		id: 'preprocessor',
		value: { type: 'identity' }
	}
	let state = emptyFlowModuleState()

	if (inlineScript) {
		;[module, state] = await createInlineScriptModule(
			inlineScript.language,
			'preprocessor',
			undefined,
			'preprocessor'
		)
	} else if (wsScript) {
		;[module, state] = await pickScript(wsScript.path, wsScript.summary, module.id, wsScript.hash)
	}

	flowStore.update((fs) => {
		fs.value.preprocessor_module = module
		return fs
	})

	flowStateStore.update((fss) => {
		fss[module.id] = state
		return fss
	})
}

export async function insertNewFailureModule(
	flowStore: Writable<ExtendedOpenFlow>,
	flowStateStore: Writable<FlowState>,
	inlineScript?: {
		language: RawScript['language']
		subkind: 'pgsql' | 'flow'
		instructions?: string
	},
	wsScript?: { path: string; summary: string; hash: string | undefined }
) {
	let module: FlowModule = {
		id: 'failure',
		value: { type: 'identity' }
	}
	let state: FlowModuleState = {
		schema: emptySchema(),
		previewResult: NEVER_TESTED_THIS_FAR
	}

	if (inlineScript) {
		;[module, state] = await createInlineScriptModule(
			inlineScript.language,
			'failure',
			inlineScript.subkind,
			'failure'
		)
	} else if (wsScript) {
		;[module, state] = await pickScript(wsScript.path, wsScript.summary, module.id, wsScript.hash)
	}

	flowStore.update((fs) => {
		fs.value.failure_module = module
		return fs
	})

	flowStateStore.update((fss) => {
		fss[module.id] = state
		return fss
	})
}

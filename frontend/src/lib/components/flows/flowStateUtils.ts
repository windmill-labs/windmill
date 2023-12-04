import type { Schema } from '$lib/common'
import {
	Script,
	ScriptService,
	type FlowModule,
	type PathFlow,
	type PathScript,
	type RawScript,
	type OpenFlow
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
		return { schema, previewResult: NEVER_TESTED_THIS_FAR }
	} catch (e) {
		console.debug(e)
		return emptyFlowModuleState()
	}
}

export async function pickScript(
	path: string,
	summary: string,
	id: string,
	hash?: string
): Promise<[FlowModule & { value: PathScript }, FlowModuleState]> {
	const flowModule: FlowModule & { value: PathScript } = {
		id,
		value: { type: 'script', path, hash, input_transforms: {} },
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
	language: RawScript.language,
	kind: Script.kind,
	subkind: 'pgsql' | 'flow',
	id: string,
	summary?: string
): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id,
		summary,
		value: { type: 'rawscript', content: code, language, input_transforms: {} }
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createLoop(id: string): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id,
		value: {
			type: 'forloopflow',
			modules: [],
			iterator: { type: 'javascript', expr: '["dynamic or static array"]' },
			skip_failures: true
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
			branches: [{ modules: [] }]
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
			language: language as RawScript.language,
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
	schema: Schema,
	flow: OpenFlow,
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
			language: flowModule.value.language
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

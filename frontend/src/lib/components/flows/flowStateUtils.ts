import type { Schema } from '$lib/common'
import { Script, ScriptService, type FlowModule, type RawScript } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import { getScriptByPath } from '$lib/utils'
import { get } from 'svelte/store'
import { flowStateStore, type FlowModuleState } from './flowState'
import { flowStore } from './flowStore'
import {
	emptyFlowModuleState,
	findNextAvailablePath,
	loadSchemaFromModule,
	NEVER_TESTED_THIS_FAR,
	numberToChars
} from './utils'

export async function loadFlowModuleState(flowModule: FlowModule): Promise<FlowModuleState> {
	try {
		const { input_transforms, schema } = await loadSchemaFromModule(flowModule)

		if (flowModule.value.type == 'script' || flowModule.value.type == 'rawscript') {
			flowModule.value.input_transforms = input_transforms
		}
		return { schema, previewResult: NEVER_TESTED_THIS_FAR }
	} catch (e) {
		return emptyFlowModuleState()
	}
}

// Computes the next available id
// TODO: We should compute the biggest id, not the length
function nextId(): string {
	const flowState = get(flowStateStore)
	return numberToChars(Object.keys(flowState).length + 1)
}
export async function pickScript({
	path,
	summary
}: {
	path: string
	summary?: string
}): Promise<[FlowModule, FlowModuleState]> {
	const flowModule: FlowModule = {
		id: nextId(),
		value: { type: 'script', path },
		summary,
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createInlineScriptModule({
	language,
	kind,
	subkind
}: {
	language: RawScript.language
	kind: Script.kind
	subkind: 'pgsql' | 'flow'
}): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id: nextId(),
		value: { type: 'rawscript', content: code, language },
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createLoop(): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id: nextId(),
		value: {
			type: 'forloopflow',
			modules: [],
			iterator: { type: 'javascript', expr: 'previous_result' },
			skip_failures: true
		},
		input_transforms: {}
	}

	const flowModuleState = await loadFlowModuleState(loopFlowModule)
	return [loopFlowModule, flowModuleState]
}

export async function createBranches(): Promise<[FlowModule, FlowModuleState]> {
	const branchesFlowModules: FlowModule = {
		id: nextId(),
		value: {
			type: 'branchone',
			branches: [],
			default: []
		},
		input_transforms: {},
		summary: ''
	}

	const flowModuleState = await loadFlowModuleState(branchesFlowModules)

	return [branchesFlowModules, flowModuleState]
}

export async function fork(flowModule: FlowModule): Promise<[FlowModule, FlowModuleState]> {
	if (flowModule.value.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
	const forkedFlowModule = await createInlineScriptModuleFromPath(flowModule.value.path ?? '')
	const flowModuleState = await loadFlowModuleState(forkedFlowModule)
	return [forkedFlowModule, flowModuleState]
}

async function createInlineScriptModuleFromPath(path: string): Promise<FlowModule> {
	const { content, language } = await getScriptByPath(path)

	return {
		id: nextId(),
		value: {
			type: 'rawscript',
			language: language as RawScript.language,
			content: content,
			path
		},
		input_transforms: {}
	}
}

export function emptyModule(): FlowModule {
	return {
		id: nextId(),
		value: { type: 'script', path: '' },
		input_transforms: {}
	}
}

export async function createScriptFromInlineScript({
	flowModule,
	suffix,
	schema
}: {
	flowModule: FlowModule
	suffix: string
	schema: Schema
}): Promise<[FlowModule, FlowModuleState]> {
	const flow = get(flowStore)
	const user = get(userStore)

	if (flowModule.value.type != 'rawscript') {
		throw new Error("Can't create script from non-inline script")
	}

	const originalScriptPath = flowModule.value.path
	const wasForked = Boolean(originalScriptPath)

	if (wasForked && originalScriptPath) {
		const [first, second, ...others] = originalScriptPath.split('/')
		suffix = others.join('/')
	}

	const path = `${flow.path}/${suffix}`
	const forkedDescription = wasForked ? `as a fork of ${originalScriptPath}` : ''
	const description = `This script was edited in place of flow ${flow.path} ${forkedDescription} by ${user?.username}.`

	const availablePath = await findNextAvailablePath(path)

	await ScriptService.createScript({
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

	return pickScript({ path: availablePath, summary: flowModule.summary })
}

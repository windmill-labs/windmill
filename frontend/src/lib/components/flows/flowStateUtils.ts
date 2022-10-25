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
const charCode = 'a'.charCodeAt(0)

// Computes the next available id
function nextId(): string {
	const flowState = get(flowStateStore)

	const keys = Object.keys(flowState)
		.filter((key) => key !== 'failure' || key.includes('branch') || key.includes('loop'))
		.map((key) => {
			const reversedKey = key.split('').reverse().join('')
			let number = 0

			for (let i = 0; i < key.length; i++) {
				const letter = reversedKey[i].charCodeAt(0) - charCode
				number += letter + 26 * i
			}

			return number
		})

	if (keys.length === 0) {
		return numberToChars(0)
	} else {
		return numberToChars(Math.max(...keys) + 1)
	}
}

export async function pickScript(
	path: string,
	summary: string,
	id: string
): Promise<[FlowModule, FlowModuleState]> {
	const flowModule: FlowModule = {
		id,
		value: { type: 'script', path },
		summary,
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createInlineScriptModule(
	language: RawScript.language,
	kind: Script.kind,
	subkind: 'pgsql' | 'flow',
	id: string
): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id,
		value: { type: 'rawscript', content: code, language },
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleState(flowModule)]
}

export async function createLoop(id: string): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id,
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

export async function createBranches(id: string): Promise<[FlowModule, FlowModuleState]> {
	const branchesFlowModules: FlowModule = {
		id,
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

export async function createBranchAll(id: string): Promise<[FlowModule, FlowModuleState]> {
	const branchesFlowModules: FlowModule = {
		id,
		value: {
			type: 'branchall',
			branches: []
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
	const forkedFlowModule = await createInlineScriptModuleFromPath(
		flowModule.value.path ?? '',
		flowModule.id
	)
	const flowModuleState = await loadFlowModuleState(forkedFlowModule)
	return [forkedFlowModule, flowModuleState]
}

async function createInlineScriptModuleFromPath(path: string, id: string): Promise<FlowModule> {
	const { content, language } = await getScriptByPath(path)

	return {
		id,
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
		value: { type: 'identity' },
		input_transforms: {}
	}
}

export async function createScriptFromInlineScript(
	flowModule: FlowModule,
	suffix: string,
	schema: Schema
): Promise<[FlowModule, FlowModuleState]> {
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

	return pickScript(availablePath, flowModule.summary ?? '', flowModule.id)
}

export function deleteFlowStateById(id: string) {
	flowStateStore.update((fss) => {
		delete fss[id]
		return fss
	})
}

import type { Schema } from '$lib/common'
import { Script, ScriptService, type FlowModule, type PathScript, type RawScript } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import { getScriptByPath } from '$lib/utils'
import { get } from 'svelte/store'
import { flowStateStore, type FlowModuleState } from './flowState'
import { flowStore } from './flowStore'
import {
	charsToNumber,
	emptyFlowModuleState,
	findNextAvailablePath,
	loadSchemaFromModule,
	NEVER_TESTED_THIS_FAR,
	numberToChars
} from './utils'
import { Mutex } from 'async-mutex'

export async function loadFlowModuleState(flowModule: FlowModule): Promise<FlowModuleState> {
	try {
		const { input_transforms, schema } = await loadSchemaFromModule(flowModule)

		if (flowModule.value.type == 'script' || flowModule.value.type == 'rawscript') {
			flowModule.value.input_transforms = input_transforms
		}
		return { schema, previewResult: NEVER_TESTED_THIS_FAR }
	} catch (e) {
		console.error(e)
		return emptyFlowModuleState()
	}
}

export const idMutex = new Mutex()

export function getNextId(currentKeys: string[]): string {
	const max = currentKeys.reduce((acc, key) => {
		if (key === 'failure' || key.includes('branch') || key.includes('loop')) {
			return acc
		} else {
			const num = charsToNumber(key)
			return Math.max(acc, num + 1)
		}
	}, 0)
	return numberToChars(max)
}

// Computes the next available id
export function nextId(): string {
	const flowState = get(flowStateStore)

	const max = Object.keys(flowState).reduce((acc, key) => {
		if (key === 'failure' || key.includes('branch') || key.includes('loop')) {
			return acc
		} else {
			const num = charsToNumber(key)
			return Math.max(acc, num + 1)
		}
	}, 0)
	return numberToChars(max)
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

export async function createInlineScriptModule(
	language: RawScript.language,
	kind: Script.kind,
	subkind: 'pgsql' | 'flow',
	id: string
): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id,
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
			branches: []
		},
		summary: ''
	}

	const flowModuleState = await loadFlowModuleState(branchesFlowModules)

	return [branchesFlowModules, flowModuleState]
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

export function emptyModule(): FlowModule {
	return {
		id: nextId(),
		value: { type: 'identity' }
	}
}

export async function createScriptFromInlineScript(
	flowModule: FlowModule,
	suffix: string,
	schema: Schema
): Promise<[FlowModule & { value: PathScript }, FlowModuleState]> {
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

export function deleteFlowStateById(id: string) {
	flowStateStore.update((fss) => {
		delete fss[id]
		return fss
	})
}

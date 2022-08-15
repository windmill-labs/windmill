import type { Schema } from '$lib/common'
import { ScriptService, type FlowModule, type RawScript } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import { emptyModule, emptySchema, getScriptByPath } from '$lib/utils'
import { get } from 'svelte/store'
import type { FlowModuleSchema } from './flowState'
import { findNextAvailablePath, flowStore } from './flowStore'
import { loadSchemaFromModule } from './utils'

export function emptyFlowModuleSchema(): FlowModuleSchema {
	return { flowModule: emptyModule(), schema: emptySchema() }
}

export async function loadFlowModuleSchema(flowModule: FlowModule): Promise<FlowModuleSchema> {
	const { input_transform, schema } = await loadSchemaFromModule(flowModule)
	flowModule.input_transform = input_transform

	debugger

	return { flowModule, schema }
}

export async function pickScript(path: string): Promise<FlowModuleSchema> {
	const flowModule: FlowModule = {
		value: { type: 'script', path },
		input_transform: {}
	}

	return await loadFlowModuleSchema(flowModule)
}

export async function createInlineScriptModule(
	language: RawScript.language
): Promise<FlowModuleSchema> {
	const code = initialCode(language, false)

	const flowModule: FlowModule = {
		value: { type: 'rawscript', content: code, language },
		input_transform: {}
	}

	return await loadFlowModuleSchema(flowModule)
}

export async function createLoop(): Promise<FlowModuleSchema> {
	const loopFlowModule: FlowModule = {
		value: {
			type: 'forloopflow',
			value: {
				modules: []
			},
			iterator: { type: 'javascript', expr: 'result' },
			skip_failures: true
		},
		input_transform: {}
	}

	const { flowModule, schema } = await loadFlowModuleSchema(loopFlowModule)

	return {
		flowModule,
		schema,
		// By default we add a empty module to the loop
		childFlowModules: [emptyFlowModuleSchema()]
	}
}

export async function fork(flowModule: FlowModule): Promise<FlowModuleSchema> {
	if (flowModule.value.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
	const fm = await createInlineScriptModuleFromPath(flowModule.value.path ?? '')

	return await loadFlowModuleSchema(fm)
}

export async function createInlineScriptModuleFromPath(path: string): Promise<FlowModule> {
	const { content, language } = await getScriptByPath(path)

	return {
		value: {
			type: 'rawscript',
			language: language as RawScript.language,
			content: content,
			path
		},
		input_transform: {}
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
}): Promise<FlowModuleSchema> {
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
	const description = `This script was edited in place of flow ${flow.path} ${forkedDescription} by ${user?.username}}.`

	const availablePath = await findNextAvailablePath(path)

	await ScriptService.createScript({
		workspace: get(workspaceStore)!,
		requestBody: {
			path: availablePath,
			summary: '',
			description,
			content: flowModule.value.content,
			parent_hash: undefined,
			schema,
			is_template: false,
			language: flowModule.value.language
		}
	})

	return pickScript(availablePath)
}

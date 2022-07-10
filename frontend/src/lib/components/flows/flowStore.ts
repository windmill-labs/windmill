import type { Schema } from '$lib/common'
import { FlowModuleValue, ScriptService, type Flow, type FlowModule } from '$lib/gen'
import { DENO_INIT_CODE, PYTHON_INIT_CODE } from '$lib/script_helpers'
import { workspaceStore } from '$lib/stores'
import { derived, get, writable } from 'svelte/store'
import { createInlineScriptModuleFromPath, getFirstStepSchema, loadSchemaFromModule } from './utils'

export const flowStore = writable<Flow>(undefined)
export const schemasStore = writable<Schema[]>([])

flowStore.subscribe((flow: Flow) => {
	if (flow) {
		const schemas = get(schemasStore)
		if (flow.value.modules.length !== schemas.length) {
			flow.value.modules.forEach((module, index) => {
				loadSchema(index)
			})
		}
	}
})

export function initFlow(flow: Flow) {
	flowStore.set(flow)
}

export const isCopyFirstStepSchemaDisabled = derived(flowStore, (flow) => {
	const modules = flow.value.modules
	const [firstModule] = modules
	return (
		modules.length === 0 || (firstModule.value.path === '' && firstModule.value.type === 'script')
	)
})

export function addModule() {
	const newModule: FlowModule = {
		value: { type: FlowModuleValue.type.SCRIPT, path: '' },
		input_transform: {}
	}

	flowStore.update((flow: Flow) => {
		flow.value.modules = flow.value.modules.concat(newModule)
		return flow
	})
}

export async function pickScript(path: string, step: number) {
	flowStore.update((flow: Flow) => {
		if (flow.value.modules[step]) {
			flow.value.modules[step].value.path = path
		}

		return flow
	})

	await loadSchema(step)
}

export async function createInlineScriptModule(language: FlowModuleValue.language, step: number) {
	const code = language === FlowModuleValue.language.DENO ? DENO_INIT_CODE : PYTHON_INIT_CODE
	flowStore.update((flow: Flow) => {
		flow.value.modules[step].value = {
			type: FlowModuleValue.type.RAWSCRIPT,
			content: code,
			language
		}

		return flow
	})
	await loadSchema(step)
}

export async function loadSchema(step: number) {
	const flow = get(flowStore)
	const module = flow.value.modules[step]

	const { input_transform, schema } = await loadSchemaFromModule(module)

	flowStore.update((flow: Flow) => {
		flow.value.modules[step].input_transform = input_transform
		return flow
	})
	schemasStore.update((schemas) => {
		schemas[step] = schema
		return schemas
	})
}

export async function fork(step: number) {
	const flow = get(flowStore)
	const flowModuleValue = flow.value.modules[step].value

	if (flowModuleValue.path) {
		const moduleValue = await createInlineScriptModuleFromPath(flowModuleValue.path)
		flowStore.update((flow: Flow) => {
			flow.value.modules[step].value = moduleValue
			return flow
		})
	}
}

export async function createScriptFromInlineScript(step: number) {
	const flow = get(flowStore)
	const schemas = get(schemasStore)
	const flowModuleValue = flow.value.modules[step].value

	const path = `u/flow/step-${step}-${Math.floor(Math.random() * 255)}`

	const newHash = await ScriptService.createScript({
		workspace: get(workspaceStore)!,
		requestBody: {
			path,
			summary: '',
			description: '',
			content: flowModuleValue.content!,
			parent_hash: undefined,
			schema: schemas[step],
			is_template: false,
			language: flowModuleValue.language!
		}
	})

	if (!newHash) {
		return
	}

	flowStore.update((flow: Flow) => {
		flow.value.modules[step].value = {
			type: FlowModuleValue.type.SCRIPT,
			path: path
		}

		return flow
	})
	await loadSchema(step)
}

export function removeModule(step: number) {
	flowStore.update((flow: Flow) => {
		flow.value.modules.splice(step, 1)
		return flow
	})
}

export async function copyFirstStepSchema() {
	const flow = get(flowStore)
	const flowSchema = await getFirstStepSchema(flow)

	flowStore.update((flow: Flow) => {
		flow.schema = flowSchema
		return flow
	})
}

export function shouldPickOfCreateScript(step: number): boolean {
	const flow = get(flowStore)
	const module = flow.value.modules[step]
	return module.value.path === '' && module.value.language === undefined
}

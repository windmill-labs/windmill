import type { Schema } from '$lib/common'
import { RawScript, ScriptService, type Flow, type FlowModule } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import { emptySchema } from '$lib/utils'
import { derived, get, writable } from 'svelte/store'
import {
	createInlineScriptModuleFromPath,
	getFirstStepSchema,
	loadSchemaFromModule,
	scrollIntoView
} from './utils'

export type FlowMode = 'push' | 'pull'

export const mode = writable<FlowMode>('push')
export const flowStore = writable<Flow>(undefined)
export const schemasStore = writable<Schema[]>([])

export function initFlow(flow: Flow) {
	const newMode = flow.value.modules[1]?.value.type === 'forloopflow' ? 'pull' : 'push'
	mode.set(newMode)
	flow = flattenForloopFlows(flow)
	flow.value.modules.forEach((mod) => {
		Object.values(mod.input_transform).forEach((inp) => {
			if (inp.type == 'javascript') {
				// for now we use the value for dynamic expression when done in the static editor so we have to resort to this

				//@ts-ignore
				inp.value = codeToStaticTemplate(inp.expr)
			}
		})
	})
	schemasStore.set([])
	flowStore.set(flow)
	// For each module in flow, we should load the corresponding schema
	flow.value.modules.forEach((_, index) => {
		loadSchema(index)
	})
}

export function codeToStaticTemplate(code?: string): string | undefined {
	if (!code) return undefined

	const lines = code
		.split('\n')
		.slice(1)
		.filter((x) => x != '')

	if (lines.length == 1) {
		const line = lines[0].trim()
		if (line[0] == '`' && line.charAt(line.length - 1) == '`') {
			return line.slice(1, line.length - 1)
		}
	}
	return undefined
}
export function flattenForloopFlows(flow: Flow): Flow {
	let newFlow: Flow = JSON.parse(JSON.stringify(flow))
	const mod = newFlow.value.modules[1]?.value
	if (mod?.type === 'forloopflow') {
		const oldModules = mod.value?.modules ?? []
		newFlow.value.modules = newFlow.value.modules.slice(0, 1)
		newFlow.value.modules.push(...oldModules)
	}
	return newFlow
}

export const isCopyFirstStepSchemaDisabled = derived(flowStore, (flow: Flow | undefined) => {
	if (flow) {
		const modules = flow.value.modules
		const [firstModule] = modules
		return (
			modules.length === 0 || (firstModule.value.type === 'script' && firstModule.value.path === '')
		)
	} else {
		return true
	}
})

export function addModule(i?: number) {
	const newModule: FlowModule = {
		value: { type: 'script', path: '' },
		input_transform: {}
	}

	flowStore.update((flow: Flow) => {
		const insertAt = i ?? flow.value.modules.length

		flow.value.modules.splice(insertAt, 0, newModule)
		flow.value.modules = flow.value.modules
		setTimeout(() => scrollIntoView(document.querySelector(`#module-${insertAt}`)), 100)

		return flow
	})

	schemasStore.update((schemas: Schema[]) => {
		if (typeof i !== 'undefined') {
			const previousSchema = schemas[i]
			const hadSchema = Boolean(previousSchema)
			if (hadSchema) {
				schemas[i] = emptySchema()
				schemas[i + 1] = previousSchema
			}
		}

		return schemas
	})
}

export async function pickScript(path: string, step: number) {
	flowStore.update((flow: Flow) => {
		if (flow.value.modules[step]) {
			flow.value.modules[step].value = { type: 'script', path }
		}

		return flow
	})

	await loadSchema(step)
}

export async function createInlineScriptModule(
	language: RawScript.language,
	step: number,
	mode: FlowMode
) {
	const code = initialCode(language, mode === 'pull' && step == 0)
	flowStore.update((flow: Flow) => {
		flow.value.modules[step].value = {
			type: 'rawscript',
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

	if (flowModuleValue.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
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
	const user = get(userStore)

	const flowModuleValue = flow.value.modules[step].value

	if (flowModuleValue.type != 'rawscript') {
		throw new Error("Can't create script from non-inline script")
	}

	const originalScriptPath = flowModuleValue.path
	const wasForked = Boolean(originalScriptPath)

	let suffix = `step-${step}`

	if (wasForked && originalScriptPath) {
		const [first, second, ...others] = originalScriptPath.split('/')
		suffix = others.join('/')
	}

	const path = `${flow.path}/${suffix}`
	const forkedDescription = wasForked ? `as a fork of ${originalScriptPath}` : ''
	const description = `This script was edited in place of flow ${flow.path} ${forkedDescription} by ${user?.username} at step ${step}.`

	const availablePath = await findNextAvailablePath(path)

	await ScriptService.createScript({
		workspace: get(workspaceStore)!,
		requestBody: {
			path: availablePath,
			summary: '',
			description,
			content: flowModuleValue.content,
			parent_hash: undefined,
			schema: schemas[step],
			is_template: false,
			language: flowModuleValue.language
		}
	})

	flowStore.update((flow: Flow) => {
		flow.value.modules[step].value = {
			type: 'script',
			path: availablePath
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

	schemasStore.update((schemas: Schema[]) => {
		schemas.splice(step, 1)
		return schemas
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

export async function findNextAvailablePath(path: string): Promise<string> {
	try {
		await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path
		})

		const [_, version] = path.split(/.*_([0-9]*)/)

		if (version.length > 0) {
			path = path.slice(0, -(version.length + 1))
		}

		path = `${path}_${Number(version) + 1}`

		return findNextAvailablePath(path)
	} catch (e) {
		// Catching an error means the path is available
		return path
	}
}

export function shouldPickOrCreateScript(flow: Flow, step: number): boolean {
	const module = flow.value.modules[step]
	return module.value.type === 'script' && module.value.path === ''
}

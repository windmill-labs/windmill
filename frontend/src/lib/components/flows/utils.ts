import type { Schema } from '$lib/common'
import {
	JobService,
	type Flow,
	type FlowModule,
	type FlowModuleValue,
	type InputTransform,
	type Job,
	type RawScript
} from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { workspaceStore } from '$lib/stores'
import { emptySchema, getScriptByPath, schemaToObject } from '$lib/utils'
import { get } from 'svelte/store'

import { mode, type FlowMode } from './flowStore'

export function flowToMode(flow: Flow | any, mode: FlowMode): Flow {
	const newFlow: Flow = JSON.parse(JSON.stringify(flow))
	newFlow.value.modules.forEach((mod) => {
		Object.values(mod.input_transform).forEach((inp) => {
			// for now we use the value for dynamic expression when done in the static editor so we have to resort to this
			if (inp.type == 'javascript') {
				//@ts-ignore
				inp.value = undefined
			} else {
				//@ts-ignore
				inp.expr = undefined
			}
		})
	})

	if (mode == 'pull') {
		const triggerModule = newFlow.value.modules[0]
		const oldModules = newFlow.value.modules.slice(1)

		if (triggerModule) {
			triggerModule.stop_after_if_expr = 'result.res1.length == 0'
			triggerModule.skip_if_stopped = true
		}

		newFlow.value.modules = newFlow.value.modules.slice(0, 1)
		if (oldModules.length > 0) {
			newFlow.value.modules.push({
				input_transform: oldModules[0].input_transform,
				value: {
					type: 'forloopflow',
					iterator: { type: 'javascript', expr: 'result.res1' },
					value: {
						modules: oldModules
					},
					skip_failures: true
				}
			})
		}
	}
	return newFlow
}

export function getTypeAsString(arg: any): string {
	if (arg === null) {
		return 'null'
	}
	if (arg === undefined) {
		return 'undefined'
	}
	return typeof arg
}

export async function getFirstStepSchema(flow: Flow): Promise<Schema> {
	const [firstModule] = flow.value.modules
	if (firstModule.value.type === 'rawscript') {
		const { language, content } = firstModule.value
		if (language && content) {
			const schema = emptySchema()
			await inferArgs(language, content, schema)
			return schema
		}
	} else if (firstModule.value.type == 'script') {
		return await loadSchema(firstModule.value.path)
	}
	return emptySchema()
}

export async function createInlineScriptModuleFromPath(path: string): Promise<FlowModuleValue> {
	const { content, language } = await getScriptByPath(path)

	return {
		type: 'rawscript',
		language: language as RawScript.language,
		content: content,
		path
	}
}

export function scrollIntoView(el: any) {
	if (!el) return

	el.scrollIntoView({
		behavior: 'smooth',
		block: 'start',
		inline: 'nearest'
	})
}
export async function loadSchemaFromModule(module: FlowModule): Promise<{
	input_transform: Record<string, InputTransform>
	schema: Schema
}> {
	const mod = module.value

	if (mod.type == 'rawscript' || mod.type === 'script') {
		let schema: Schema
		if (mod.type === 'rawscript') {
			schema = emptySchema()
			await inferArgs(mod.language!, mod.content!, schema)
		} else {
			schema = await loadSchema(mod.path!)
		}

		const keys = Object.keys(schema?.properties ?? {})

		let input_transform = module.input_transform

		if (
			JSON.stringify(keys.sort()) !== JSON.stringify(Object.keys(module.input_transform).sort())
		) {
			input_transform = keys.reduce((accu, key) => {
				let nv = module.input_transform[key] ?? {
					type: 'static',
					value: undefined
				}
				accu[key] = nv
				return accu
			}, {})
		}

		return {
			input_transform: input_transform,
			schema: schema ?? emptySchema()
		}
	}

	return {
		input_transform: {},
		schema: emptySchema()
	}
}

export function isCodeInjection(expr: string | undefined): boolean {
	if (!expr) {
		return false
	}
	const lines = expr.split('\n')
	const [returnStatement] = lines.reverse()

	const returnStatementRegex = new RegExp(/\$\{(.*)\}/)
	if (returnStatementRegex.test(returnStatement)) {
		const [_, argName] = returnStatement.split(returnStatementRegex)

		return Boolean(argName)
	}
	return false
}

export function getDefaultExpr(i: number, key: string = 'myfield', previousExpr?: string) {
	const expr = previousExpr ?? `previous_result.${key}`
	return `import { previous_result, flow_input, step, variable, resource, params } from 'windmill@${i}'

${expr}`
}

export function getPickableProperties(
	schema: Schema,
	args: Record<string, any>,
	previewResults: Record<number, Object>,
	mode: FlowMode,
	i: number
) {
	const flowInputAsObject = schemaToObject(schema, args)
	const flowInput =
		mode === 'pull' && i >= 1
			? computeFlowInputPull(previewResults[0], flowInputAsObject)
			: flowInputAsObject

	let previous_result
	if (i === 0 || (i == 1 && mode == 'pull')) {
		previous_result = flowInput
	} else if (mode == 'pull') {
		previous_result = previewResults[1] ? previewResults[1][i - 2] : undefined
	} else {
		previous_result = previewResults[i - 1]
	}

	let step: any[]
	if (i >= 1 && mode == 'push') {
		step = Object.values(previewResults).slice(0, i)
	} else if (i >= 2 && mode == 'pull') {
		step = Object.values(previewResults[1] ?? {}).slice(0, i - 1)
	} else {
		step = []
	}
	const pickableProperties = {
		flow_input: flowInput,
		previous_result,
		step
	}

	return pickableProperties
}

export function jobsToResults(jobs: Job[]) {
	return jobs.map((job) => {
		if ('result' in job) {
			return job.result
		} else if (Array.isArray(job)) {
			return jobsToResults(job)
		}
	})
}

export async function runFlowPreview(args: Record<string, any>, flow: Flow) {
	const newFlow = flowToMode(flow, get(mode))
	return await JobService.runFlowPreview({
		workspace: get(workspaceStore) ?? '',
		requestBody: {
			args,
			value: newFlow.value,
			path: newFlow.path
		}
	})
}
function computeFlowInputPull(previewResult: any | undefined, flowInputAsObject: any) {
	const iteratorValues = (previewResult?.res1 && Array.isArray(previewResult.res1)) ?
		{
			_value: previewResult.res1[0],
			_index: `The current index of the iteration as a number (here from 0 to ${previewResult.res1.length - 1})`
		} : {
			_value: 'The current value of the iteration as an object',
			_index: 'The current index of the iteration as a number'
		}
	return Object.assign(Object.assign(flowInputAsObject, previewResult), iteratorValues)

}


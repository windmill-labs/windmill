import type { Schema } from '$lib/common'
import { JobService, type Flow, type FlowModule, type InputTransform, type Job } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { workspaceStore } from '$lib/stores'
import { emptySchema, schemaToObject } from '$lib/utils'
import { get } from 'svelte/store'

import type { FlowMode } from './flowStore'

export function cleanInputs(flow: Flow | any): Flow {
	const newFlow: Flow = JSON.parse(JSON.stringify(flow))
	newFlow.value.modules.forEach((mod) => {
		Object.values(mod.input_transform).forEach((inp) => {
			// for now we use the value for dynamic expression when done in the static editor so we have to resort to this
			if (inp.type == 'javascript') {
				//@ts-ignore
				inp.value = undefined
				inp.expr = inp.expr
					.split('\n')
					.filter(
						(x) =>
							x != '' &&
							!x.startsWith(
								`import { previous_result, flow_input, step, variable, resource, params } from 'windmill@`
							)
					)
					.join('\n')
			} else {
				//@ts-ignore
				inp.expr = undefined
			}
		})
	})

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

export function scrollIntoView(element: any) {
	if (!element) return

	element.scrollIntoView({
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

const returnStatementRegex = new RegExp(/\$\{(.*)\}/)

export function isCodeInjection(expr: string | undefined): boolean {
	if (!expr) {
		return false
	}
	const lines = expr.split('\n')
	const [returnStatement] = lines.reverse()

	return returnStatementRegex.test(returnStatement)
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
	const newFlow = flow
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
	const iteratorValues =
		previewResult && Array.isArray(previewResult)
			? {
				iter: {
					value: previewResult[0],
					index: `The current index of the iteration as a number (here from 0 to ${previewResult.length - 1
						})`
				}
			}
			: {
				iter: {
					value: 'The current value of the iteration as an object',
					index: 'The current index of the iteration as a number'
				}
			}
	return Object.assign(flowInputAsObject, iteratorValues)
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

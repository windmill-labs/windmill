import type { Schema } from '$lib/common'
import { JobService, type Flow, type FlowModule, type InputTransform, type Job } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { workspaceStore } from '$lib/stores'
import { emptySchema } from '$lib/utils'
import { get } from 'svelte/store'

export function cleanInputs(flow: Flow | any): Flow {
	const newFlow: Flow = JSON.parse(JSON.stringify(flow))
	newFlow.value.modules.forEach((mod) => {
		Object.values(mod.input_transforms).forEach((inp) => {
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


export function selectedIdToIndexes(selectedId: string): number[] {
	const splitted = selectedId.split('-')
	if (splitted[0] == 'loop') {
		return [Number(splitted[1])]
	} else {
		return splitted.map(Number)
	}
}
export function selectedIdToModule(selectedId: string, flow: Flow): FlowModule {
	const [p, c] = selectedIdToIndexes(selectedId)
	const pm = flow.value.modules[p]
	if (c && pm.value.type == 'forloopflow') {
		return pm.value.modules[c]
	} else {
		return pm
	}
}


export async function loadSchemaFromModule(module: FlowModule): Promise<{
	input_transforms: Record<string, InputTransform>
	schema: Schema
}> {
	const mod = module.value

	if (mod.type == 'rawscript' || mod.type === 'script') {
		let schema: Schema
		if (mod.type === 'rawscript') {
			schema = emptySchema()
			await inferArgs(mod.language!, mod.content ?? '', schema)
		} else if (mod.path && mod.path != '') {
			schema = await loadSchema(mod.path!)
		} else {
			return {
				input_transforms: {},
				schema: emptySchema()
			}
		}

		const keys = Object.keys(schema?.properties ?? {})

		let input_transforms = module.input_transforms

		if (
			JSON.stringify(keys.sort()) !== JSON.stringify(Object.keys(module.input_transforms).sort())
		) {
			input_transforms = keys.reduce((accu, key) => {
				let nv = module.input_transforms[key] ?? {
					type: 'static',
					value: undefined
				}
				accu[key] = nv
				return accu
			}, {})
		}

		return {
			input_transforms: input_transforms,
			schema: schema ?? emptySchema()
		}
	}

	return {
		input_transforms: {},
		schema: emptySchema()
	}
}

const dynamicTemplateRegex = new RegExp(/\$\{(.*)\}/)

export function isCodeInjection(expr: string | undefined): boolean {
	if (!expr) {
		return false
	}

	return dynamicTemplateRegex.test(expr)
}

export function getDefaultExpr(
	importPath: string | undefined = undefined,
	key: string = 'myfield',
	previousExpr?: string
) {
	const expr = previousExpr ?? `previous_result.${key}`
	return `import { previous_result, flow_input, step, variable, resource, params } from 'windmill${importPath ? `@${importPath}` : ''
		}'

${expr}`
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

export function getIndexes(parentIndex: number | undefined, childIndex: number): number[] {
	const indexes: number[] = []

	if (parentIndex !== undefined) {
		indexes.push(parentIndex)
	}
	indexes.push(childIndex)

	return indexes
}

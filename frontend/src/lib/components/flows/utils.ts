import type { Schema } from '$lib/common'
import {
	JobService,
	ScriptService,
	type Flow,
	type FlowModule,
	type InputTransform,
	type Job
} from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { workspaceStore } from '$lib/stores'
import { emptySchema } from '$lib/utils'
import { get } from 'svelte/store'
import type { FlowModuleState } from './flowState'

export function cleanInputs(flow: Flow | any): Flow {
	const newFlow: Flow = JSON.parse(JSON.stringify(flow))
	newFlow.value.modules.forEach((mod) => {
		if (mod.value.type == 'rawscript' || mod.value.type == 'script') {
			if (Object.keys(mod.input_transforms ?? {}).length > 0) {
				mod.value.input_transforms = mod.input_transforms!
				delete mod.input_transforms
			}
			Object.values(mod.input_transforms ?? {}).forEach((inp) => {
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
		}
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

		if (Object.keys(module.input_transforms ?? {}).length > 0) {
			mod.input_transforms = module.input_transforms!
		}
		let input_transforms = mod.input_transforms ?? module.input_transforms ?? {}

		if (JSON.stringify(keys.sort()) !== JSON.stringify(Object.keys(input_transforms).sort())) {
			input_transforms = keys.reduce((accu, key) => {
				let nv = input_transforms[key] ?? {
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

export const NEVER_TESTED_THIS_FAR = 'never tested this far'

export function emptyFlowModuleState(): FlowModuleState {
	return {
		schema: emptySchema(),
		previewResult: NEVER_TESTED_THIS_FAR
	}
}

const aCharCode = 'a'.charCodeAt(0)

export function numberToChars(n: number) {
	var b = [n],
		sp,
		out,
		i,
		div

	sp = 0
	while (sp < b.length) {
		if (b[sp] > 25) {
			div = Math.floor(b[sp] / 26)
			b[sp + 1] = div - 1
			b[sp] %= 26
		}
		sp += 1
	}

	out = ''
	for (i = 0; i < b.length; i += 1) {
		out = String.fromCharCode(aCharCode + b[i]) + out
	}

	return out
}

export function charsToNumber(n: string): number {
	let b = Math.pow(26, n.length - 1)
	let res = 0
	for (let c of n) {
		res += (c.charCodeAt(0) - aCharCode + 1) * b
		b = b / 26
	}
	return res - 1
}

export function isEmptyFlowModule(flowModule: FlowModule): boolean {
	return flowModule.value.type === 'identity'
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

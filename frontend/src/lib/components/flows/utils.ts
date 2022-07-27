import type { Schema } from '$lib/common'
import { FlowModuleValue, InputTransform, type Flow, type FlowModule } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { emptySchema, getScriptByPath, schemaToObject } from '$lib/utils'

import type { FlowMode } from './flowStore'

export function flowToMode(flow: Flow | any, mode: FlowMode): Flow {
	if (mode == 'pull') {
		const newFlow: Flow = JSON.parse(JSON.stringify(flow))
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
					type: FlowModuleValue.type.FORLOOPFLOW,
					iterator: { type: InputTransform.type.JAVASCRIPT, expr: 'result.res1' },
					value: {
						modules: oldModules
					},
					skip_failures: true
				}
			})
		}
		return newFlow
	}
	return flow
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
	if (firstModule.value.type === FlowModuleValue.type.RAWSCRIPT) {
		const { language, content } = firstModule.value
		if (language && content) {
			const schema = emptySchema()
			await inferArgs(language, content, schema)
			return schema
		}
	} else if (firstModule.value.path) {
		return await loadSchema(firstModule.value.path)
	}
	return emptySchema()
}

export async function createInlineScriptModuleFromPath(path: string): Promise<FlowModuleValue> {
	const { content, language } = await getScriptByPath(path)

	return {
		type: FlowModuleValue.type.RAWSCRIPT,
		language: language as FlowModuleValue.language,
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
	const isRaw = module?.value.type === FlowModuleValue.type.RAWSCRIPT

	if (isRaw || Boolean(module.value.path)) {
		let schema: Schema
		if (isRaw) {
			schema = emptySchema()
			await inferArgs(module.value.language!, module.value.content!, schema)
		} else {
			schema = await loadSchema(module.value.path!)
		}

		const keys = Object.keys(schema?.properties ?? {})

		let input_transform = module.input_transform

		if (
			JSON.stringify(Object.keys(schema?.properties ?? {}).sort()) !==
			JSON.stringify(Object.keys(module.input_transform).sort())
		) {
			input_transform = keys.reduce((accu, key) => {
				accu[key] = {
					type: 'static',
					value: undefined
				}
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

export function getCodeInjectionExpr(code: string, isRaw: boolean): string {
	let expr = `\`${code}\``
	if (isRaw) {
		expr = `JSON.parse(${expr})`
	}
	return `import { previous_result, flow_input, step, variable, resource, params } from 'windmill'
${expr}`
}

export function getDefaultExpr(i: number, key: string = 'myfield', previousExpr?: string) {
	const expr = previousExpr ? `\`${previousExpr}\`` : `previous_result.${key}`
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
			? Object.assign(Object.assign(
				{
					_value: 'The current value of the iteration.',
					_index: 'The current index of the iteration.'
				},
				flowInputAsObject), previewResults[0])

			: flowInputAsObject


	let previous_result
	if (i === 0 || (i == 1 && mode == 'pull')) {
		previous_result = flowInput
	} else if (mode == 'pull') {
		previous_result = previewResults[1] ? previewResults[1][i - 2] : undefined
	} else {
		previous_result = previewResults[i - 1]
	}

	let step
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

import type { Schema } from '$lib/common'
import {
	FlowModuleValue,
	InputTransform,
	ScriptService,
	type Flow,
	type FlowModule
} from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { loadSchema } from '$lib/scripts'
import { DENO_INIT_CODE, PYTHON_INIT_CODE } from '$lib/script_helpers'
import { workspaceStore } from '$lib/stores'
import { emptySchema } from '$lib/utils'
import { get } from 'svelte/store'

function filterByKey(obj: Object, key: string): Object {
	if (Object(obj) !== obj) {
		return obj
	} else if (Array.isArray(obj)) {
		return obj.map((o) => filterByKey(o, key))
	} else {
		return Object.fromEntries(
			Object.entries(obj)
				.filter(([k, v]) => !k.includes(key))
				.map(([k, v]) => [k, filterByKey(v, key)])
		)
	}
}

function diff(target: Object, source: Object): Object {
	if (Array.isArray(target)) {
		return target
	}

	const result = {}

	Object.keys(target).forEach((key: string) => {
		if (typeof source[key] === 'object') {
			const difference = diff(target[key], source[key])

			if (Object.keys(difference).length > 0) {
				result[key] = difference
			}
		} else if (source[key] !== target[key]) {
			result[key] = target[key]
		}
	})

	return result
}

export function keepByKey(json: Object, key: string): Object {
	return diff(json, filterByKey(json, key))
}

export function getTypeAsString(arg: any): string {
	if (arg === null) {
		return 'null'
	}
	return typeof arg
}

export function formatValue(arg: any) {
	if (getTypeAsString(arg) === 'string') {
		return `"${arg}"`
	}
	return arg
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

export function createInlineScriptModule(language: FlowModuleValue.language): FlowModuleValue {
	const code = language === FlowModuleValue.language.DENO ? DENO_INIT_CODE : PYTHON_INIT_CODE

	return {
		type: FlowModuleValue.type.RAWSCRIPT,
		content: code,
		language
	}
}

export async function getScriptByPath(path: string): Promise<{
	content: string
	language: FlowModuleValue.language
}> {
	if (path.startsWith('hub/')) {
		const content = await ScriptService.getHubScriptContentByPath({ path })
		return {
			content,
			language: FlowModuleValue.language.DENO
		}
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return {
			content: script.content,
			language: script.language
		}
	}
}

export async function createInlineScriptModuleFromPath(path: string): Promise<FlowModuleValue> {
	const { content, language } = await getScriptByPath(path)

	return {
		type: FlowModuleValue.type.RAWSCRIPT,
		language: language,
		content: content
	}
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
		const inputTransform = keys.reduce((accu, key) => {
			accu[key] = {
				type: 'static',
				value: ''
			}
			return accu
		}, {})

		return {
			input_transform: inputTransform,
			schema: schema ?? emptySchema()
		}
	}

	return {
		input_transform: {},
		schema: emptySchema()
	}
}

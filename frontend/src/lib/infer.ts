import { ScriptService, type MainArgSignature, FlowService, Script } from '$lib/gen'
import { get, writable } from 'svelte/store'
import type { Schema, SchemaProperty, SupportedLanguage } from './common.js'
import { emptySchema, sortObject } from './utils.js'
import { tick } from 'svelte'
import init, {
	parse_deno,
	parse_bash,
	parse_go,
	parse_python,
	parse_sql,
	parse_mysql,
	parse_bigquery,
	parse_snowflake,
	parse_graphql,
	parse_powershell,
	parse_outputs,
	parse_mssql
} from 'windmill-parser-wasm'
import wasmUrl from 'windmill-parser-wasm/windmill_parser_wasm_bg.wasm?url'
import { workspaceStore } from './stores.js'

init(wasmUrl)

const loadSchemaLastRun = writable<[string | undefined, MainArgSignature | undefined]>(undefined)

export async function inferArgs(
	language: SupportedLanguage,
	code: string,
	schema: Schema
): Promise<void> {
	await init(wasmUrl)
	const lastRun = get(loadSchemaLastRun)
	let inferedSchema: MainArgSignature
	if (lastRun && code == lastRun[0] && lastRun[1]) {
		inferedSchema = lastRun[1]
	} else {
		if (code == '') {
			code = ' '
		}
		if (language == 'python3') {
			inferedSchema = JSON.parse(parse_python(code))
		} else if (language == 'deno') {
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'nativets') {
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'bun') {
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'postgresql') {
			inferedSchema = JSON.parse(parse_sql(code))
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'postgresql' } },
				...inferedSchema.args
			]
		} else if (language == 'mysql') {
			inferedSchema = JSON.parse(parse_mysql(code))
			inferedSchema.args = [{ name: 'database', typ: { resource: 'mysql' } }, ...inferedSchema.args]
		} else if (language == 'bigquery') {
			inferedSchema = JSON.parse(parse_bigquery(code))
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'bigquery' } },
				...inferedSchema.args
			]
		} else if (language == 'snowflake') {
			inferedSchema = JSON.parse(parse_snowflake(code))
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'snowflake' } },
				...inferedSchema.args
			]
		} else if (language == 'mssql') {
			inferedSchema = JSON.parse(parse_mssql(code))
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'ms_sql_server' } },
				...inferedSchema.args
			]
		} else if (language == 'graphql') {
			inferedSchema = JSON.parse(parse_graphql(code))
			inferedSchema.args = [{ name: 'api', typ: { resource: 'graphql' } }, ...inferedSchema.args]
		} else if (language == 'go') {
			inferedSchema = JSON.parse(parse_go(code))
		} else if (language == 'bash') {
			inferedSchema = JSON.parse(parse_bash(code))
		} else if (language == 'powershell') {
			inferedSchema = JSON.parse(parse_powershell(code))
		} else {
			return
		}
		if (inferedSchema.type == 'Invalid') {
			throw new Error(inferedSchema.error)
		}
		loadSchemaLastRun.set([code, inferedSchema])
	}

	schema.required = []
	const oldProperties = JSON.parse(JSON.stringify(schema.properties))
	schema.properties = {}

	for (const arg of inferedSchema.args) {
		if (!(arg.name in oldProperties)) {
			schema.properties[arg.name] = { description: '', type: '' }
		} else {
			schema.properties[arg.name] = oldProperties[arg.name]
		}
		schema.properties[arg.name] = sortObject(schema.properties[arg.name])

		argSigToJsonSchemaType(arg.typ, schema.properties[arg.name])

		schema.properties[arg.name].default = arg.default

		if (!arg.has_default && !schema.required.includes(arg.name)) {
			schema.required.push(arg.name)
		}
	}
	await tick()
}

function argSigToJsonSchemaType(
	t:
		| string
		| { resource: string | null }
		| { list: string | { str: any } | { object: { key: string; typ: any }[] } | null }
		| { str: string[] | null }
		| { object: { key: string; typ: any }[] },
	oldS: SchemaProperty
): void {
	const newS: SchemaProperty = { type: '' }
	if (t === 'int') {
		newS.type = 'integer'
	} else if (t === 'float') {
		newS.type = 'number'
	} else if (t === 'bool') {
		newS.type = 'boolean'
	} else if (t === 'email') {
		newS.type = 'string'
		newS.format = 'email'
	} else if (t === 'sql') {
		newS.type = 'string'
		newS.format = 'sql'
	} else if (t === 'yaml') {
		newS.type = 'string'
		newS.format = 'yaml'
	} else if (t === 'bytes') {
		newS.type = 'string'
		newS.contentEncoding = 'base64'
	} else if (t === 'datetime') {
		newS.type = 'string'
		newS.format = 'date-time'
	} else if (typeof t !== 'string' && `object` in t) {
		newS.type = 'object'
		if (t.object) {
			const properties = {}
			for (const prop of t.object) {
				properties[prop.key] = {}
				argSigToJsonSchemaType(prop.typ, properties[prop.key])
			}
			newS.properties = properties
		}
	} else if (typeof t !== 'string' && `str` in t) {
		newS.type = 'string'
		if (t.str) {
			newS.enum = t.str
		}
	} else if (typeof t !== 'string' && `resource` in t) {
		newS.type = 'object'
		newS.format = `resource-${t.resource}`
	} else if (typeof t !== 'string' && `list` in t) {
		newS.type = 'array'
		if (t.list === 'int' || t.list === 'float') {
			newS.items = { type: 'number' }
		} else if (t.list === 'bytes') {
			newS.items = { type: 'string', contentEncoding: 'base64' }
		} else if (t.list == 'string') {
			newS.items = { type: 'string' }
		} else if (t.list && typeof t.list == 'object' && 'str' in t.list) {
			newS.items = { type: 'string', enum: t.list.str }
		} else {
			newS.items = { type: 'object' }
		}
	} else {
		newS.type = 'object'
	}

	if (oldS.type != newS.type) {
		for (const prop of Object.getOwnPropertyNames(newS)) {
			if (prop != 'description') {
				delete oldS[prop]
			}
		}
	} else if (oldS.format == 'date-time' && newS.format != 'date-time') {
		delete oldS.format
	} else if (oldS.items?.type != newS.items?.type) {
		delete oldS.items
	}

	let sameItems = oldS.items?.type == 'string' && newS.items?.type == 'string'
	let savedItems: any = undefined
	if (sameItems) {
		savedItems = JSON.parse(JSON.stringify(oldS.items))
	}
	Object.assign(oldS, newS)
	if (sameItems) {
		oldS.items = savedItems
	}

	if (oldS.format?.startsWith('resource-') && newS.type != 'object') {
		oldS.format = undefined
	}
}

export async function loadSchemaFromPath(path: string, hash?: string): Promise<Schema> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })
		if (language == 'deno' || language == 'nativets') {
			const newSchema = emptySchema()
			await inferArgs('deno' as SupportedLanguage, content ?? '', newSchema)
			return newSchema
		} else {
			return schema ?? emptySchema()
		}
	} else if (hash) {
		const script = await ScriptService.getScriptByHash({
			workspace: get(workspaceStore)!,
			hash
		})
		return inferSchemaIfNecessary(script)
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return inferSchemaIfNecessary(script)
	}
}

async function inferSchemaIfNecessary(script: Script) {
	if (script.schema) {
		return script.schema as any
	} else {
		const newSchema = emptySchema()
		await inferArgs(script.language, script.content ?? '', newSchema)
		return newSchema
	}
}

export async function loadSchema(
	workspace: string,
	path: string,
	runType: 'script' | 'flow' | 'hubscript'
): Promise<{ schema: Schema; summary: string | undefined }> {
	if (runType === 'script') {
		const script = await ScriptService.getScriptByPath({
			workspace,
			path
		})

		return { schema: script.schema as any, summary: script.summary }
	} else if (runType === 'flow') {
		const flow = await FlowService.getFlowByPath({
			workspace,
			path
		})

		return { schema: flow.schema as any, summary: flow.summary }
	} else {
		const script = await ScriptService.getHubScriptByPath({
			path
		})
		if (
			script.schema == undefined ||
			Object.keys(script.schema).length == 0 ||
			typeof script.schema != 'object'
		) {
			script.schema = emptySchema()
		}

		await inferArgs(script.language as SupportedLanguage, script.content, script.schema)
		return { schema: script.schema, summary: script.summary }
	}
}

export async function parseOutputs(
	code: string,
	ignoreError
): Promise<[string, string][] | undefined> {
	await init(wasmUrl)
	const getOutputs = await parse_outputs(code)
	const outputs = JSON.parse(getOutputs)
	if (outputs.error) {
		if (ignoreError) {
			return undefined
		}
		throw new Error(outputs.error)
	}
	return outputs.error ? [] : outputs.outputs
}

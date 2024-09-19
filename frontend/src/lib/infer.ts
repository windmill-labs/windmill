import { ScriptService, type MainArgSignature, FlowService, type Script } from '$lib/gen'
import { get, writable } from 'svelte/store'
import type { Schema, SupportedLanguage } from './common.js'
import { emptySchema, sortObject } from './utils.js'
import { tick } from 'svelte'

import initTsParser, { parse_deno, parse_outputs } from 'windmill-parser-wasm-ts'
import initRegexParsers, {
	parse_sql,
	parse_mysql,
	parse_bigquery,
	parse_snowflake,
	parse_graphql,
	parse_mssql,
	parse_db_resource,
	parse_bash,
	parse_powershell
} from 'windmill-parser-wasm-regex'
import initPythonParser, { parse_python } from 'windmill-parser-wasm-py'
import initGoParser, { parse_go } from 'windmill-parser-wasm-go'
import initPhpParser, { parse_php } from 'windmill-parser-wasm-php'
import initRustParser, { parse_rust } from 'windmill-parser-wasm-rust'
import initYamlParser, { parse_ansible } from 'windmill-parser-wasm-yaml'

import wasmUrlTs from 'windmill-parser-wasm-ts/windmill_parser_wasm_bg.wasm?url'
import wasmUrlRegex from 'windmill-parser-wasm-regex/windmill_parser_wasm_bg.wasm?url'
import wasmUrlPy from 'windmill-parser-wasm-py/windmill_parser_wasm_bg.wasm?url'
import wasmUrlGo from 'windmill-parser-wasm-go/windmill_parser_wasm_bg.wasm?url'
import wasmUrlPhp from 'windmill-parser-wasm-php/windmill_parser_wasm_bg.wasm?url'
import wasmUrlRust from 'windmill-parser-wasm-rust/windmill_parser_wasm_bg.wasm?url'
import wasmUrlYaml from 'windmill-parser-wasm-yaml/windmill_parser_wasm_bg.wasm?url'
import { workspaceStore } from './stores.js'
import { argSigToJsonSchemaType } from './inferArgSig.js'

const loadSchemaLastRun = writable<[string | undefined, MainArgSignature | undefined]>(undefined)

let initializeTsPromise: Promise<any> | undefined = undefined
export async function initWasmTs() {
	if (initializeTsPromise == undefined) {
		initializeTsPromise = initTsParser(wasmUrlTs)
	}
	await initializeTsPromise
}
async function initWasmRegex() {
	await initRegexParsers(wasmUrlRegex)
}
async function initWasmPython() {
	await initPythonParser(wasmUrlPy)
}
async function initWasmPhp() {
	await initPhpParser(wasmUrlPhp)
}
async function initWasmRust() {
	await initRustParser(wasmUrlRust)
}
async function initWasmGo() {
	await initGoParser(wasmUrlGo)
}
async function initWasmYaml() {
	await initYamlParser(wasmUrlYaml)
}

export async function inferArgs(
	language: SupportedLanguage | 'bunnative' | undefined,
	code: string,
	schema: Schema
): Promise<boolean | null> {
	const lastRun = get(loadSchemaLastRun)
	let inferedSchema: MainArgSignature
	if (lastRun && code == lastRun[0] && lastRun[1]) {
		inferedSchema = lastRun[1]
	} else {
		if (code == '') {
			code = ' '
		}

		let inlineDBResource: string | undefined = undefined
		if (['postgresql', 'mysql', 'bigquery', 'snowflake', 'mssql'].includes(language ?? '')) {
			await initWasmRegex()
			inlineDBResource = parse_db_resource(code)
		}
		if (language == 'python3') {
			await initWasmPython()
			inferedSchema = JSON.parse(parse_python(code))
		} else if (language == 'deno') {
			await initWasmTs()
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'nativets') {
			await initWasmTs()
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'bun' || language == 'bunnative') {
			await initWasmTs()
			inferedSchema = JSON.parse(parse_deno(code))
		} else if (language == 'postgresql') {
			inferedSchema = JSON.parse(parse_sql(code))
			if (inlineDBResource === undefined) {
				inferedSchema.args = [
					{
						name: 'database',
						typ: { resource: 'postgresql' }
					},
					...inferedSchema.args
				]
			}
		} else if (language == 'mysql') {
			inferedSchema = JSON.parse(parse_mysql(code))
			if (inlineDBResource === undefined) {
				inferedSchema.args = [
					{ name: 'database', typ: { resource: 'mysql' } },
					...inferedSchema.args
				]
			}
		} else if (language == 'bigquery') {
			inferedSchema = JSON.parse(parse_bigquery(code))
			if (inlineDBResource === undefined) {
				inferedSchema.args = [
					{ name: 'database', typ: { resource: 'bigquery' } },
					...inferedSchema.args
				]
			}
		} else if (language == 'snowflake') {
			inferedSchema = JSON.parse(parse_snowflake(code))
			if (inlineDBResource === undefined) {
				inferedSchema.args = [
					{ name: 'database', typ: { resource: 'snowflake' } },
					...inferedSchema.args
				]
			}
		} else if (language == 'mssql') {
			inferedSchema = JSON.parse(parse_mssql(code))
			if (inlineDBResource === undefined) {
				inferedSchema.args = [
					{ name: 'database', typ: { resource: 'ms_sql_server' } },
					...inferedSchema.args
				]
			}
		} else if (language == 'graphql') {
			await initWasmRegex()
			inferedSchema = JSON.parse(parse_graphql(code))
			inferedSchema.args = [{ name: 'api', typ: { resource: 'graphql' } }, ...inferedSchema.args]
		} else if (language == 'go') {
			await initWasmGo()
			inferedSchema = JSON.parse(parse_go(code))
		} else if (language == 'bash') {
			await initWasmRegex()
			inferedSchema = JSON.parse(parse_bash(code))
		} else if (language == 'powershell') {
			await initWasmRegex()
			inferedSchema = JSON.parse(parse_powershell(code))
		} else if (language == 'php') {
			await initWasmPhp()
			inferedSchema = JSON.parse(parse_php(code))
		} else if (language == 'rust') {
			await initWasmRust()
			inferedSchema = JSON.parse(parse_rust(code))
		} else if (language == 'ansible') {
			await initWasmYaml()
			inferedSchema = JSON.parse(parse_ansible(code))
		} else {
			return null
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

	return inferedSchema.no_main_func
}

export async function loadSchemaFromPath(path: string, hash?: string): Promise<Schema> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })

		if (schema && typeof schema === 'object' && 'properties' in schema) {
			return schema as any
		} else {
			const newSchema = emptySchema()
			await inferArgs(language as SupportedLanguage, content ?? '', newSchema)
			return newSchema
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

		await inferArgs(script.language as SupportedLanguage, script.content, script.schema as any)
		return { schema: script.schema as any, summary: script.summary }
	}
}

export async function parseOutputs(
	code: string,
	ignoreError
): Promise<[string, string][] | undefined> {
	await initWasmTs()
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

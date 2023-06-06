import { ScriptService, type MainArgSignature } from '$lib/gen'
import { get, writable } from 'svelte/store'
import type { Schema, SchemaProperty } from './common.js'
import { sortObject } from './utils.js'
import { tick } from 'svelte'
import init, { parse_deno_wasm } from 'windmill-parser-ts-wasm'
import wasmUrl from 'windmill-parser-ts-wasm/windmill_parser_ts_wasm_bg.wasm?url'

init(wasmUrl)

const loadSchemaLastRun = writable<[string | undefined, MainArgSignature | undefined]>(undefined)

export async function inferArgs(
	language: 'python3' | 'deno' | 'go' | 'bash',
	code: string,
	schema: Schema
): Promise<void> {
	const lastRun = get(loadSchemaLastRun)
	let inferedSchema: MainArgSignature
	if (lastRun && code == lastRun[0] && lastRun[1]) {
		inferedSchema = lastRun[1]
	} else {
		if (code == '') {
			code = ' '
		}
		if (language == 'python3') {
			inferedSchema = await ScriptService.pythonToJsonschema({
				requestBody: code
			})
		} else if (language == 'deno') {
			inferedSchema = JSON.parse(parse_deno_wasm(code))
		} else if (language == 'go') {
			inferedSchema = await ScriptService.goToJsonschema({
				requestBody: code
			})
		} else if (language == 'bash') {
			inferedSchema = await ScriptService.bashToJsonschema({
				requestBody: code
			})
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
		} else if (t.list && typeof t.list == 'object' && `object` in t.list) {
			newS.items = { type: 'object' }
		} else {
			newS.items = { type: 'string' }
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
	}

	Object.assign(oldS, newS)
	if (oldS.format?.startsWith('resource-') && newS.type != 'object') {
		oldS.format = undefined
	}
}

import { Schema, SchemaProperty, SupportedLanguage, MainArgSignature, ArgType } from './types'

// Re-export types for consumers
export * from './types'

// Import parsers
import initTsParser, { parse_deno } from 'windmill-parser-wasm-ts'
import initRegexParsers, {
	parse_sql,
	parse_mysql,
	parse_oracledb,
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
import initCSharpParser, { parse_csharp } from 'windmill-parser-wasm-csharp'
import initNuParser, { parse_nu } from 'windmill-parser-wasm-nu'
import initJavaParser, { parse_java } from 'windmill-parser-wasm-java'

// Import WASM URLs
import wasmUrlTs from 'windmill-parser-wasm-ts/windmill_parser_wasm_bg.wasm?url'
import wasmUrlRegex from 'windmill-parser-wasm-regex/windmill_parser_wasm_bg.wasm?url'
import wasmUrlPy from 'windmill-parser-wasm-py/windmill_parser_wasm_bg.wasm?url'
import wasmUrlGo from 'windmill-parser-wasm-go/windmill_parser_wasm_bg.wasm?url'
import wasmUrlPhp from 'windmill-parser-wasm-php/windmill_parser_wasm_bg.wasm?url'
import wasmUrlRust from 'windmill-parser-wasm-rust/windmill_parser_wasm_bg.wasm?url'
import wasmUrlYaml from 'windmill-parser-wasm-yaml/windmill_parser_wasm_bg.wasm?url'
import wasmUrlCSharp from 'windmill-parser-wasm-csharp/windmill_parser_wasm_bg.wasm?url'
import wasmUrlNu from 'windmill-parser-wasm-nu/windmill_parser_wasm_bg.wasm?url'
import wasmUrlJava from 'windmill-parser-wasm-java/windmill_parser_wasm_bg.wasm?url'

let initializeTsPromise: Promise<any> | undefined = undefined
export async function initWasmTs() {
	if (initializeTsPromise == undefined) {
		initializeTsPromise = initTsParser(wasmUrlTs)
	}
	await initializeTsPromise
}

async function initWasmRegex(): Promise<void> {
	await initRegexParsers(wasmUrlRegex)
}

async function initWasmPython(): Promise<void> {
	await initPythonParser(wasmUrlPy)
}

async function initWasmGo(): Promise<void> {
	await initGoParser(wasmUrlGo)
}

async function initWasmPhp(): Promise<void> {
	await initPhpParser(wasmUrlPhp)
}

async function initWasmRust(): Promise<void> {
	await initRustParser(wasmUrlRust)
}

async function initWasmYaml(): Promise<void> {
	await initYamlParser(wasmUrlYaml)
}

async function initWasmCSharp(): Promise<void> {
	await initCSharpParser(wasmUrlCSharp)
}

async function initWasmNu(): Promise<void> {
	await initNuParser(wasmUrlNu)
}

async function initWasmJava(): Promise<void> {
	await initJavaParser(wasmUrlJava)
}

// Helper function to convert arg signature to JSON Schema type
export function argSigToJsonSchemaType(t: ArgType, oldS: SchemaProperty): void {
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
		newS.originalType = 'bytes'
	} else if (t === 'datetime') {
		newS.type = 'string'
		newS.format = 'date-time'
	} else if (typeof t !== 'string' && 'oneof' in t) {
		newS.type = 'object'
		if (t.oneof) {
			newS.oneOf = t.oneof.map((obj) => {
				const oldObjS = oldS.oneOf?.find((o) => o?.title === obj.label)
				const properties: Record<string, SchemaProperty> = {}
				for (const prop of obj.properties) {
					if (oldObjS?.properties && prop.key in oldObjS?.properties) {
						properties[prop.key] = oldObjS?.properties[prop.key]
					} else {
						properties[prop.key] = { description: '', type: '' }
					}
					argSigToJsonSchemaType(prop.typ, properties[prop.key])
				}
				return {
					type: 'object',
					title: obj.label,
					properties,
					order: oldObjS?.order
				}
			})
		}
	} else if (typeof t !== 'string' && 'object' in t) {
		newS.type = 'object'
		if (t.object) {
			const properties: Record<string, SchemaProperty> = {}
			for (const prop of t.object) {
				if (oldS.properties && prop.key in oldS.properties) {
					properties[prop.key] = oldS.properties[prop.key]
				} else {
					properties[prop.key] = { description: '', type: '' }
				}
				argSigToJsonSchemaType(prop.typ, properties[prop.key])
			}
			newS.properties = properties
		}
	} else if (typeof t !== 'string' && 'str' in t) {
		newS.type = 'string'
		if (t.str) {
			newS.originalType = 'enum'
			newS.enum = t.str
		} else if (oldS.originalType == 'string' && oldS.enum) {
			newS.originalType = 'string'
			newS.enum = oldS.enum
		} else {
			newS.originalType = 'string'
			newS.enum = undefined
		}
	} else if (typeof t !== 'string' && 'resource' in t) {
		newS.type = 'object'
		newS.format = `resource-${t.resource}`
	} else if (typeof t !== 'string' && 'dynselect' in t) {
		newS.type = 'object'
		newS.format = `dynselect-${t.dynselect}`
	} else if (typeof t !== 'string' && 'list' in t) {
		newS.type = 'array'
		if (t.list === 'int' || t.list === 'float') {
			newS.items = { type: 'number' }
			newS.originalType = 'number[]'
		} else if (t.list === 'bytes') {
			newS.items = { type: 'string', contentEncoding: 'base64' }
			newS.originalType = 'bytes[]'
		} else if (t.list && typeof t.list == 'object' && 'str' in t.list && t.list.str) {
			newS.items = { type: 'string', enum: t.list.str }
			newS.originalType = 'enum[]'
		} else if (t.list == 'string' || (t.list && typeof t.list == 'object' && 'str' in t.list)) {
			newS.items = { type: 'string', enum: oldS.items?.enum }
			newS.originalType = 'string[]'
		} else if (t.list && typeof t.list == 'object' && 'resource' in t.list && t.list.resource) {
			newS.items = {
				type: 'resource',
				resourceType: t.list.resource as string
			}
			newS.originalType = 'resource[]'
		} else if (
			t.list &&
			typeof t.list == 'object' &&
			'object' in t.list &&
			t.list.object &&
			t.list.object.length > 0
		) {
			const properties: Record<string, SchemaProperty> = {}
			for (const prop of t.list.object) {
				properties[prop.key] = { description: '', type: '' }

				argSigToJsonSchemaType(prop.typ, properties[prop.key])
			}

			newS.items = { type: 'object', properties: properties }
			newS.originalType = 'record[]'
		} else {
			newS.items = { type: 'object' }
			newS.originalType = 'object[]'
		}
	} else {
		newS.type = 'object'
	}

	const preservedFields = [
		'description',
		'pattern',
		'min',
		'max',
		'currency',
		'currencyLocale',
		'multiselect',
		'customErrorMessage',
		'required',
		'showExpr',
		'password',
		'order',
		'dateFormat',
		'title',
		'placeholder'
	]

	preservedFields.forEach((field) => {
		if (oldS[field as keyof SchemaProperty] !== undefined) {
			;(newS as any)[field] = (oldS as any)[field]
		}
	})

	if (oldS.type != newS.type) {
		for (const prop of Object.getOwnPropertyNames(newS)) {
			if (prop != 'description') {
				delete (oldS as any)[prop]
			}
		}
	} else if ((oldS.format == 'date' || oldS.format === 'date-time') && newS.format == 'string') {
		newS.format = oldS.format
	} else if (newS.format == 'date-time' && oldS.format == 'date') {
		newS.format = 'date'
	} else if (oldS.items?.type != newS.items?.type) {
		delete oldS.items
	}

	Object.assign(oldS, newS)

	if (oldS.format?.startsWith('resource-') && newS.type != 'object') {
		oldS.format = undefined
	}
}

// Helper function to sort object properties
function sortObject<T extends Record<string, any>>(obj: T): T {
	return Object.keys(obj)
		.sort()
		.reduce((result: any, key) => {
			result[key] = obj[key]
			return result
		}, {}) as T
}

// Main export - Making this a named export to ensure it's properly exported
export async function inferArgs(
	language: SupportedLanguage | 'bunnative' | undefined,
	code: string,
	schema: Schema,
	mainOverride?: string
): Promise<{
	no_main_func: boolean | null
	has_preprocessor: boolean | null
} | null> {
	let inferedSchema: MainArgSignature
	if (code == '') {
		code = ' '
	}

	let inlineDBResource: string | undefined = undefined
	if (
		['postgresql', 'mysql', 'bigquery', 'snowflake', 'mssql', 'oracledb'].includes(language ?? '')
	) {
		await initWasmRegex()
		inlineDBResource = parse_db_resource(code)
	}
	if (language == 'python3') {
		await initWasmPython()
		inferedSchema = JSON.parse(parse_python(code, mainOverride))
	} else if (language == 'deno') {
		await initWasmTs()
		inferedSchema = JSON.parse(parse_deno(code, mainOverride))
	} else if (language == 'nativets') {
		await initWasmTs()
		inferedSchema = JSON.parse(parse_deno(code, mainOverride))
	} else if (language == 'bun' || language == 'bunnative') {
		await initWasmTs()
		inferedSchema = JSON.parse(parse_deno(code, mainOverride))
	} else if (language == 'postgresql') {
		inferedSchema = JSON.parse(parse_sql(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{
					name: 'database',
					typ: { resource: 'postgresql' },
					has_default: false
				},
				...inferedSchema.args
			]
		}
	} else if (language == 'mysql') {
		inferedSchema = JSON.parse(parse_mysql(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'mysql' }, has_default: false },
				...inferedSchema.args
			]
		}
	} else if (language == 'bigquery') {
		inferedSchema = JSON.parse(parse_bigquery(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'bigquery' }, has_default: false },
				...inferedSchema.args
			]
		}
	} else if (language == 'oracledb') {
		inferedSchema = JSON.parse(parse_oracledb(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'oracledb' }, has_default: false },
				...inferedSchema.args
			]
		}
	} else if (language == 'snowflake') {
		inferedSchema = JSON.parse(parse_snowflake(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'snowflake' }, has_default: false },
				...inferedSchema.args
			]
		}
	} else if (language == 'mssql') {
		inferedSchema = JSON.parse(parse_mssql(code))
		if (inlineDBResource === undefined) {
			inferedSchema.args = [
				{ name: 'database', typ: { resource: 'ms_sql_server' }, has_default: false },
				...inferedSchema.args
			]
		}
	} else if (language == 'graphql') {
		await initWasmRegex()
		inferedSchema = JSON.parse(parse_graphql(code))
		inferedSchema.args = [
			{ name: 'api', typ: { resource: 'graphql' }, has_default: false },
			...inferedSchema.args
		]
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
	} else if (language == 'csharp') {
		await initWasmCSharp()
		inferedSchema = JSON.parse(parse_csharp(code))
	} else if (language == 'nu') {
		await initWasmNu()
		inferedSchema = JSON.parse(parse_nu(code))
	} else if (language == 'java') {
		await initWasmJava()
		inferedSchema = JSON.parse(parse_java(code))
		// for related places search: ADD_NEW_LANG
	} else {
		return null
	}
	if (inferedSchema.type == 'Invalid') {
		throw new Error(inferedSchema.error)
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
	return {
		no_main_func: inferedSchema.no_main_func,
		has_preprocessor: inferedSchema.has_preprocessor
	}
}

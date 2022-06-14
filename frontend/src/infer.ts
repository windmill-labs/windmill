import type { Schema, SchemaProperty } from './common'
import { ScriptService, type MainArgSignature } from './gen'
import { sendUserToast } from './utils'

export async function inferArgs(
	language: 'python3' | 'deno',
	code: string,
	schema: Schema
): Promise<void> {
	try {
		let inferedSchema: MainArgSignature
		if (language == 'python3') {
			inferedSchema = await ScriptService.pythonToJsonschema({
				requestBody: code
			})
		} else if (language == 'deno') {
			inferedSchema = await ScriptService.denoToJsonschema({
				requestBody: code
			})
		} else {
			return
		}

		schema.required = []
		const oldProperties = Object.assign({}, schema.properties)
		schema.properties = {}

		for (const arg of inferedSchema.args) {
			if (!(arg.name in oldProperties)) {
				schema.properties[arg.name] = { description: '', type: '' }
			} else {
				schema.properties[arg.name] = oldProperties[arg.name]
			}
			argSigToJsonSchemaType(arg.typ, schema.properties[arg.name])
			schema.properties[arg.name].default = arg.default

			if (!arg.has_default) {
				schema.required.push(arg.name)
			}
		}
	} catch (err) {
		console.error(err)
		sendUserToast(`Could not infer schema: ${err.body ?? err}`, true)
	}
}

function argSigToJsonSchemaType(t: string | { resource: string }, s: SchemaProperty): void {
	if (t === 'int') {
		s.type = 'integer'
	} else if (t === 'float') {
		s.type = 'number'
	} else if (t === 'bool') {
		s.type = 'boolean'
	} else if (t === 'str') {
		s.type = 'string'
	} else if (t === 'dict') {
		s.type = 'object'
	} else if (t === 'list') {
		s.type = 'array'
	} else if (t === 'bytes') {
		s.type = 'string'
		s.contentEncoding = 'base64'
	} else if (t === 'datetime') {
		s.type = 'string'
		s.format = 'date-time'
	} else if (typeof t !== 'string' && t.resource != undefined) {
		s.type = 'object'
		s.format = `resource-${t.resource}`
	} else {
		s.type = undefined
	}
}

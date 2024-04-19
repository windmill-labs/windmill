import type { SchemaProperty } from './common'

export function argSigToJsonSchemaType(
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

	Object.assign(oldS, newS)

	// if (sameItems && savedItems != undefined && savedItems.enum != undefined) {
	// 	sendUserToast(JSON.stringify(savedItems))
	// 	oldS.items = savedItems
	// }

	if (oldS.format?.startsWith('resource-') && newS.type != 'object') {
		oldS.format = undefined
	}
}

import type { SchemaProperty } from './common'

export function argSigToJsonSchemaType(
	t:
		| string
		| { resource: string | null }
		| {
				list:
					| (string | { object: { key: string; typ: any }[] })
					| { str: any }
					| { object: { key: string; typ: any }[] }
					| null
		  }
		| { dynselect: string }
		| { str: string[] | null }
		| { object: { key: string; typ: any }[] }
		| {
				oneof: [
					{
						label: string
						properties: { key: string; typ: any }[]
					}
				]
		  },
	oldS: SchemaProperty
): void {
	let newS: SchemaProperty = { type: '' }
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
				const oldObjS = oldS.oneOf?.find((o) => o?.title === obj.label) ?? undefined
				const properties = {}
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
					order: oldObjS?.order ?? undefined
				}
			})
		}
	} else if (typeof t !== 'string' && `object` in t) {
		newS.type = 'object'
		if (t.object) {
			const properties = {}
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
	} else if (typeof t !== 'string' && `str` in t) {
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
	} else if (typeof t !== 'string' && `resource` in t) {
		newS.type = 'object'
		newS.format = `resource-${t.resource}`
	} else if (typeof t !== 'string' && `dynselect` in t) {
		newS.type = 'object'
		newS.format = `dynselect-${t.dynselect}`
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
		} else if (t.list && typeof t.list == 'object' && 'resource' in t.list && t.list.resource) {
			newS.items = { type: 'resource', resourceType: t.list.resource as string }
		} else if (
			t.list &&
			typeof t.list == 'object' &&
			'object' in t.list &&
			t.list.object &&
			t.list.object.length > 0
		) {
			const properties = {}
			for (const prop of t.list.object) {
				properties[prop.key] = { description: '', type: '' }

				argSigToJsonSchemaType(prop.typ, properties[prop.key])
			}

			newS.items = { type: 'object', properties: properties }
		} else {
			newS.items = { type: 'object' }
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
		if (oldS[field] !== undefined) {
			newS[field] = oldS[field]
		}
	})

	if (oldS.type != newS.type) {
		for (const prop of Object.getOwnPropertyNames(newS)) {
			if (prop != 'description') {
				delete oldS[prop]
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

	// if (sameItems && savedItems != undefined && savedItems.enum != undefined) {
	// 	sendUserToast(JSON.stringify(savedItems))
	// 	oldS.items = savedItems
	// }

	if (oldS.format?.startsWith('resource-') && newS.type != 'object') {
		oldS.format = undefined
	}
}

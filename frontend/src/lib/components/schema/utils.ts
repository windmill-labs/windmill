import type { SchemaProperty, ModalSchemaProperty } from '$lib/common'

export function testRegex(pattern: string, value: any): boolean {
	try {
		const regex = new RegExp(pattern)
		return regex.test(value)
	} catch (err) {
		return false
	}
}

export function evalValueToRaw(
	inputCategory: string,
	value: any,
	isListJson: boolean
): string | undefined {
	return inputCategory === 'object' ||
		inputCategory === 'resource-object' ||
		(inputCategory == 'list' && !isListJson)
		? JSON.stringify(value, null, 2)
		: undefined
}

export function schemaToModal(
	schema: SchemaProperty,
	name: string,
	required: boolean
): ModalSchemaProperty {
	return {
		name,
		selectedType: schema.type,
		description: schema.description ?? '',
		pattern: schema.pattern,
		default: schema.default,
		contentEncoding: schema.contentEncoding,
		format: schema.format,
		enum_: schema.enum,
		min: schema.min,
		max: schema.max,
		currency: schema.currency,
		currencyLocale: schema.currencyLocale,
		multiselect: schema.multiselect,
		items: schema.items?.type
			? { type: schema.items.type as 'string' | 'number' | undefined, enum: schema.items.enum }
			: undefined,
		required,
		schema:
			schema.type == 'object'
				? {
						$schema: undefined,
						type: schema.type,
						properties: schema.properties ?? {},
						required: schema.required ?? []
				  }
				: undefined,
		showExpr: schema.showExpr,
		password: schema.password,
		nullable: schema.nullable,
		dateFormat: schema.format,
		title: schema.title,
		placeholder: schema.placeholder
	}
}

import type { Script, ScriptLang } from './gen'

export type OwnerKind = 'group' | 'user' | 'folder'

export type ActionKind = 'Create' | 'Update' | 'Delete' | 'Execute'

export type SupportedLanguage = Script['language']

export interface PropertyDisplayInfo {
	property: SchemaProperty
	name: string
	isRequired: boolean
	path: string[]
	index: number
	propertiesNumber: number
}

export type EnumType = string[] | { value: string; label: string }[] | undefined

export interface SchemaProperty {
	type: string | undefined
	description?: string
	pattern?: string
	default?: any
	enum?: EnumType
	contentEncoding?: 'base64' | 'binary'
	format?: string
	items?: {
		type?: 'string' | 'number' | 'bytes' | 'object' | 'resource'
		contentEncoding?: 'base64'
		enum?: string[]
		resourceType?: string
		properties?: { [name: string]: SchemaProperty }
	}
	min?: number
	max?: number
	currency?: string
	currencyLocale?: string
	multiselect?: boolean
	customErrorMessage?: string
	properties?: { [name: string]: SchemaProperty }
	required?: string[]
	showExpr?: string
	password?: boolean
	order?: string[]
	nullable?: boolean
	dateFormat?: string
	title?: string
	placeholder?: string
	oneOf?: SchemaProperty[]
	originalType?: string
	disabled?: boolean
	'x-no-s3-storage-workspace-warning'?: string
}

export interface ModalSchemaProperty {
	selectedType?: string
	description: string
	name: string
	required: boolean
	min?: number
	max?: number
	currency?: string
	currencyLocale?: string
	multiselect?: boolean
	format?: string
	pattern?: string
	enum_?: EnumType
	default?: any
	items?: { type?: 'string' | 'number'; enum?: string[] }
	contentEncoding?: 'base64' | 'binary'
	schema?: Schema
	customErrorMessage?: string
	showExpr?: string
	password?: boolean
	nullable?: boolean
	dateFormat?: string
	title?: string
	placeholder?: string
	oneOf?: SchemaProperty[]
}

export function modalToSchema(schema: ModalSchemaProperty): SchemaProperty {
	return {
		type: schema.selectedType,
		description: schema.description,
		pattern: schema.pattern,
		default: schema.default,
		enum: schema.enum_,
		items: schema.items,
		contentEncoding: schema.contentEncoding,
		format: schema.format,
		customErrorMessage: schema.customErrorMessage,
		properties: schema.schema?.properties,
		required: schema.schema?.required,
		min: schema.min,
		max: schema.max,
		currency: schema.currency,
		currencyLocale: schema.currencyLocale,
		multiselect: schema.multiselect,
		showExpr: schema.showExpr,
		password: schema.password,
		nullable: schema.nullable,
		dateFormat: schema.dateFormat,
		title: schema.title,
		placeholder: schema.placeholder,
		oneOf: schema.oneOf
	}
}
export type Schema = {
	$schema: string | undefined
	type: string
	"x-windmill-dyn-select-code"?: string
	"x-windmill-dyn-select-lang"?: ScriptLang
	properties: { [name: string]: SchemaProperty }
	order?: string[]
	required: string[]
}

export function mergeSchema(
	schema: Schema | Record<string, any>,
	enum_payload: Record<string, any> = {}
) {
	if (!schema.properties || !enum_payload) {
		return schema
	}
	let new_schema: Schema = JSON.parse(JSON.stringify(schema))
	for (let [key, value] of Object.entries(new_schema.properties ?? {})) {
		if (enum_payload[key]) {
			value.enum = enum_payload[key]
			value['disableCreate'] = true
		}
	}

	return new_schema
}

export type Meta = { ownerKind: OwnerKind; owner: string; name: string }

type Enumerate<N extends number, Acc extends number[] = []> = Acc['length'] extends N
	? Acc[number]
	: Enumerate<N, [...Acc, Acc['length']]>

/** An inclusive range of integer numbers */
export type IntRange<F extends number, T extends number> =
	| F
	| Exclude<Enumerate<T>, Enumerate<F>>
	| T

export function pathToMeta(path: string, hideUser: boolean): Meta {
	const splitted = path.split('/')
	let ownerKind: OwnerKind
	if (splitted[0] == 'g') {
		ownerKind = 'group'
	} else if (splitted[0] == 'f') {
		ownerKind = 'folder'
	} else if (splitted[0] == 'u') {
		ownerKind = 'user'
	} else {
		console.error('Not recognized owner:' + splitted[0])
		return {
			ownerKind: hideUser ? 'folder' : 'user',
			owner: '',
			name: ''
		}
	}
	return {
		ownerKind,
		owner: splitted[1],
		name: splitted.slice(2).join('/')
	}
}

export function prettyLanguage(lang: string) {
	switch (lang) {
		case 'nativets':
			return 'Native TypeScript'
		default:
			return lang.charAt(0).toUpperCase() + lang.slice(1)
	}
}

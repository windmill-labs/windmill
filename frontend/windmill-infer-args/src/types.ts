// Type definitions for Windmill infer args

export type SchemaProperty = {
	type: string
	description?: string
	format?: string
	contentEncoding?: string
	originalType?: string
	oneOf?: SchemaOneOf[]
	properties?: Record<string, SchemaProperty>
	items?: {
		type: string
		enum?: string[]
		contentEncoding?: string
		resourceType?: string
		properties?: Record<string, SchemaProperty>
	}
	enum?: string[]
	pattern?: string
	min?: number
	max?: number
	currency?: string
	currencyLocale?: string
	multiselect?: boolean
	customErrorMessage?: string
	required?: boolean
	showExpr?: string
	password?: boolean
	order?: number
	dateFormat?: string
	title?: string
	placeholder?: string
	default?: any
}

export type SchemaOneOf = {
	type: string
	title?: string
	properties?: Record<string, SchemaProperty>
	order?: number
}

export type Schema = {
	$schema: string | undefined
	type: string
	properties: {
		[name: string]: SchemaProperty
	}
	order?: string[]
	required: string[]
}

export type TypeObject = {
	key: string
	typ: ArgType
}

export type ArgType =
	| string
	| { resource: string | null }
	| {
			list:
				| (string | { object: TypeObject[] })
				| { str: any }
				| { object: TypeObject[] }
				| { resource: string | null }
				| null
	  }
	| { dynselect: string }
	| { str: string[] | null }
	| { object: TypeObject[] }
	| {
			oneof: [
				{
					label: string
					properties: TypeObject[]
				}
			]
	  }

export type MainArgSignature = {
	type: string
	error?: string
	no_main_func: boolean | null
	has_preprocessor: boolean | null
	args: {
		name: string
		typ: ArgType
		default?: any
		has_default: boolean
	}[]
}

export type SupportedLanguage =
	| 'python3'
	| 'deno'
	| 'nativets'
	| 'bun'
	| 'postgresql'
	| 'mysql'
	| 'bigquery'
	| 'oracledb'
	| 'snowflake'
	| 'mssql'
	| 'graphql'
	| 'go'
	| 'bash'
	| 'powershell'
	| 'php'
	| 'rust'
	| 'ansible'
	| 'csharp'
	| 'nu'
	| 'java'

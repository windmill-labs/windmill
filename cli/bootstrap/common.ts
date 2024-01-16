
export interface SchemaProperty {
	type: string | undefined
	description?: string
	pattern?: string
	default?: any
	enum?: string[]
	contentEncoding?: 'base64' | 'binary'
	format?: string
	items?: {
		type?: 'string' | 'number' | 'bytes' | 'object'
		contentEncoding?: 'base64'
		enum?: string[]
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
}

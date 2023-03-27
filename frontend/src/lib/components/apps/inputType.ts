import type { ReadFileAs } from '../common/fileInput/model'
import type { InlineScript } from './types'

export type InputType =
	| 'text'
	| 'textarea'
	| 'template'
	| 'number'
	| 'boolean'
	| 'select'
	| 'icon-select'
	| 'color'
	| 'date'
	| 'time'
	| 'datetime'
	| 'object'
	| 'array'
	| 'any'
	| 'labeledresource'
	| 'labeledselect'
	| 'tab-select'

// Connection to an output of another component
// defined by the id of the component and the path of the output
export type InputConnection = {
	componentId: string
	path: string
}

// Connected input, connected to an output of another component by the developer
export type ConnectedInput = {
	type: 'connected'
	connection: InputConnection | undefined
}

// User input, set by the user in the app
export type UserInput<U> = {
	type: 'user'
	value: U | undefined
}

// Input can be uploaded with a file selector
export type UploadInput = {
	type: 'upload'
	value: string
}

export type EvalInput = {
	type: 'eval'
	expr: string
}

export type RowInput = {
	type: 'row'
	column: string
}

// Static input, set by the developer in the component panel
export type StaticInput<U> = {
	value: U | undefined
	type: 'static'
}

export type TemplateInput = {
	eval: string
	type: 'template'
}

export type RunnableByPath = {
	path: string
	schema: any
	runType: 'script' | 'flow' | 'hubscript'
	type: 'runnableByPath'
}

export type RunnableByName = {
	name: string
	inlineScript: InlineScript | undefined
	type: 'runnableByName'
}

export type Runnable = RunnableByPath | RunnableByName | undefined

// Runnable input, set by the developer in the component panel
export type ResultInput = {
	runnable: Runnable
	transformer?: InlineScript & { language: 'frontend' }
	fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	type: 'runnable'
	value?: any
	doNotRecomputeOnInputChanged?: boolean
}

type AppInputSpec<T extends InputType, U, V extends InputType = never> = (
	| StaticInput<U>
	| ConnectedInput
	| UserInput<U>
	| RowInput
	| EvalInput
	| UploadInput
	| ResultInput
	| TemplateInput
) &
	InputConfiguration<T, V>

type InputConfiguration<T extends InputType, V extends InputType> = {
	fieldType: T
	subFieldType?: V
	format?: string | undefined
	fileUpload?: {
		/** Use `*` to accept anything. */
		accept: string
		/**
		 * Controls if user is allowed to select multiple files.
		 * @default false
		 */
		multiple?: boolean
		/**
		 * Controls if the uploaded file(s) will be converted or not.
		 * @default false
		 */
		convertTo?: ReadFileAs
	}
}

export type StaticOptions = {
	selectOptions: readonly string[] | readonly { value: string; label: string }[]
}

export type AppInput =
	| AppInputSpec<'text', string>
	| AppInputSpec<'textarea', string>
	| AppInputSpec<'template', string>
	| AppInputSpec<'number', number>
	| AppInputSpec<'boolean', boolean>
	| AppInputSpec<'date', string>
	| AppInputSpec<'time', string>
	| AppInputSpec<'datetime', string>
	| AppInputSpec<'any', any>
	| AppInputSpec<'object', Record<string | number, any>>
	| AppInputSpec<'object', string>
	| (AppInputSpec<'select', string> & StaticOptions)
	| AppInputSpec<'icon-select', string>
	| AppInputSpec<'color', string>
	| AppInputSpec<'array', string[], 'text'>
	| AppInputSpec<'array', string[], 'textarea'>
	| AppInputSpec<'array', number[], 'number'>
	| AppInputSpec<'array', boolean[], 'boolean'>
	| AppInputSpec<'array', string[], 'date'>
	| AppInputSpec<'array', string[], 'time'>
	| AppInputSpec<'array', string[], 'datetime'>
	| AppInputSpec<'array', object[], 'object'>
	| (AppInputSpec<'array', string[], 'select'> & StaticOptions)
	| AppInputSpec<'array', object[], 'labeledresource'>
	| AppInputSpec<'array', object[], 'labeledselect'>
	| AppInputSpec<'labeledselect', object>
	| AppInputSpec<'labeledresource', object>
	| AppInputSpec<'array', object[], 'tab-select'>

export type RowAppInput = Extract<AppInput, { type: 'row' }>
export type StaticAppInput = Extract<AppInput, { type: 'static' }>
export type ConnectedAppInput = Extract<AppInput, { type: 'connected' }>
export type UserAppInput = Extract<AppInput, { type: 'user' }>
export type ResultAppInput = Extract<AppInput, { type: 'runnable' }>
export type EvalAppInput = Extract<AppInput, { type: 'eval' }>
export type UploadAppInput = Extract<AppInput, { type: 'upload' }>

export type RichAppInput =
	| AppInput
	| { type: 'oneOf'; oneOf: string[]; configuration: Record<string, AppInput> }
	| { type: 'group'; title: string; configuration: Record<string, AppInput> }

export type AppInputs = Record<string, AppInput>

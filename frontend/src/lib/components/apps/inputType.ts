import type { ReadFileAs } from '../common/fileInput/model'
import type { DecisionTreeNode, TypedComponent } from './editor/component'
import type { InlineScript } from './sharedTypes'
export type { InlineScript } from './sharedTypes'

export type InputType =
	| 'integer'
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
	| 'schema'
	| 'ag-grid'
	| 'table-column'
	| 'plotly'
	| 'chartjs'
	| 'DecisionTreeNode'
	| 'ag-chart'
	| 'resource'
	| 'db-explorer'
	| 'db-table'
	| 's3'
	| 'number-tuple'
	// Used for selecting the right resource type in the Database Studio
	| 'postgres'
	| 'mysql'
	| 'ms_sql_server'
	| 'snowflake'
	| 'snowflake_oauth'
	| 'bigquery'
	| 'oracledb'
	| 'ducklake'
	| 'datatable'
	| 'app-path'

// Connection to an output of another component
// defined by the id of the component and the path of the output
export type InputConnection = {
	componentType?: TypedComponent['type']
	componentId: string
	path: string
}

export type InputConnectionEval = {
	componentId: string
	id: string
}

// Connected input, connected to an output of another component by the developer
export type ConnectedInput = {
	type: 'connected'
	connection: InputConnection | undefined
	allowUserResources?: boolean
}

// User input, set by the user in the app
export type UserInput<U> = {
	type: 'user'
	value: U | undefined
	allowUserResources?: boolean
}

// Input can be uploaded with a file selector
export type UploadInput = {
	type: 'upload'
	value: string
}

export type UploadS3Input = {
	type: 'uploadS3'
	value: any
}

export type FileUploadData = {
	name: string
	size: number
	progress: number
	cancelled?: boolean
	errorMessage?: string
	path?: string
	file?: File
}

export type EvalInput = {
	type: 'eval'
	expr: string
}

export type EvalInputV2 = {
	type: 'evalv2'
	expr: string
	connections: InputConnectionEval[]
	onDemandOnly?: boolean
	allowUserResources?: boolean
}

// Context input, secure backend-resolved value (username, email, groups, workspace, author)
export type CtxInput = {
	type: 'ctx'
	ctx: 'username' | 'email' | 'groups' | 'workspace' | 'author'
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

export type TemplateV2Input = {
	eval: string
	type: 'templatev2'
	connections: InputConnectionEval[]
}

export type RunnableByPath = {
	name: string
	path: string
	schema: any
	runType: 'script' | 'flow' | 'hubscript'
	type: 'runnableByPath' | 'path'
}

export function isRunnableByPath(runnable: Runnable): runnable is RunnableByPath {
	return runnable?.type == 'runnableByPath' || runnable?.type == 'path'
}

export function isRunnableByName(runnable: Runnable): runnable is RunnableByName {
	return runnable?.type == 'runnableByName' || runnable?.type == 'inline'
}

export type RunnableByName = {
	name: string
	inlineScript: InlineScript | undefined
	type: 'runnableByName' | 'inline'
}

export type Runnable = RunnableByPath | RunnableByName | undefined

export type RunnableWithFields = Runnable & {
	fields?: Record<string, StaticAppInput | UserAppInput | CtxInput>
}

// Runnable input, set by the developer in the component panel
export type ResultInput = {
	runnable: Runnable
	transformer?: InlineScript & { language: 'frontend' }
	fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	type: 'runnable'
	value?: any
	// kept for migration purposes
	doNotRecomputeOnInputChanged?: boolean
	recomputeOnInputChanged?: boolean
	autoRefresh?: boolean
	hideRefreshButton?: boolean
}

export type AppInputSpec<T extends InputType, U, V extends InputType = never> = (
	| StaticInput<U>
	| ConnectedInput
	| UserInput<U>
	| RowInput
	| EvalInput
	| EvalInputV2
	| UploadInput
	| UploadS3Input
	| ResultInput
	| TemplateInput
	| TemplateV2Input
) &
	InputConfiguration<T, V>

type InputConfiguration<T extends InputType, V extends InputType> = {
	fieldType: T
	subFieldType?: V
	format?: string | undefined
	loading?: boolean
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
	fileUploadS3?: {
		accept: string
		multiple?: boolean
	}
	noStatic?: boolean
	onDemandOnly?: boolean
	hideRefreshButton?: boolean
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
	| (AppInputSpec<'select', string, 'db-table'> & StaticOptions)
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
	| AppInputSpec<'array', (object | string)[], 'labeledselect'>
	| AppInputSpec<'labeledselect', object>
	| AppInputSpec<'labeledresource', object>
	| AppInputSpec<'array', object[], 'tab-select'>
	| AppInputSpec<'schema', object>
	| AppInputSpec<'array', object[], 'ag-grid'>
	| AppInputSpec<'array', object[], 'db-explorer'>
	| AppInputSpec<'array', object[], 'table-column'>
	| AppInputSpec<'array', object[], 'plotly'>
	| AppInputSpec<'array', object[], 'chartjs'>
	| AppInputSpec<'array', DecisionTreeNode, 'DecisionTreeNode'>
	| AppInputSpec<'array', object[], 'ag-chart'>
	| AppInputSpec<'resource', string>
	| AppInputSpec<'resource', string, 's3'>
	| AppInputSpec<'resource', string, 'postgres'>
	| AppInputSpec<'resource', string, 'mysql'>
	| AppInputSpec<'resource', string, 'ms_sql_server'>
	| AppInputSpec<'resource', string, 'snowflake'>
	| AppInputSpec<'resource', string, 'snowflake_oauth'>
	| AppInputSpec<'resource', string, 'bigquery'>
	| AppInputSpec<'resource', string, 'oracledb'>
	| AppInputSpec<'ducklake', string, 'ducklake'>
	| AppInputSpec<'datatable', string, 'datatable'>
	| AppInputSpec<'array', object[], 'number-tuple'>
	| AppInputSpec<'app-path', string>

export type RowAppInput = Extract<AppInput, { type: 'row' }>
export type StaticAppInput = Extract<AppInput, { type: 'static' }>
export type ConnectedAppInput = Extract<AppInput, { type: 'connected' }>
export type UserAppInput = Extract<AppInput, { type: 'user' }>
export type ResultAppInput = Extract<AppInput, { type: 'runnable' }>
export type EvalAppInput = Extract<AppInput, { type: 'eval' }>
export type EvalV2AppInput = Extract<AppInput, { type: 'evalv2' }>
export type StaticAppInputOnDemand = Extract<StaticAppInput, { onDemandOnly: true }>
export type TemplateV2AppInput = Extract<AppInput, { type: 'templatev2' }>

export type UploadAppInput = Extract<AppInput, { type: 'upload' }>
export type UploadS3AppInput = Extract<AppInput, { type: 'uploadS3' }>
export type CtxAppInput = CtxInput

export type RichAppInput =
	| AppInput
	| { type: 'oneOf'; oneOf: string[]; configuration: Record<string, AppInput> }
	| { type: 'group'; title: string; configuration: Record<string, AppInput> }

export type AppInputs = Record<string, AppInput>

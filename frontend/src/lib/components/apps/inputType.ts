import type { staticValues } from './editor/componentsPanel/componentStaticValues'

export type InputType =
	| 'text'
	| 'textarea'
	| 'number'
	| 'boolean'
	| 'select'
	| 'date'
	| 'time'
	| 'datetime'
	| 'object'

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

// Static input, set by the developer in the component panel
export type StaticInput<U> = {
	value: U | undefined
	type: 'static'
	visible?: boolean | undefined
}

type RunnableByPath = {
	path: string
	runType: 'script' | 'flow'
	type: 'runnableByPath'
}

type RunnableByName = {
	inlineScriptName: string
	type: 'runnableByName'
}

export type Runnable = RunnableByPath | RunnableByName | undefined

// Runnable input, set by the developer in the component panel
export type ResultInput = {
	runnable: Runnable
	fields: AppInputs
	type: 'runnable'
}

type AppInputSpec<T, U> = (StaticInput<U> | ConnectedInput | UserInput<U> | ResultInput) &
	InputConfiguration<T, U>

type InputConfiguration<T, U> = {
	fieldType: T
	defaultValue: U
}

export type AppInput =
	| AppInputSpec<'text', string>
	| AppInputSpec<'textarea', string>
	| AppInputSpec<'number', number>
	| AppInputSpec<'boolean', boolean>
	| AppInputSpec<'date', string>
	| AppInputSpec<'time', string>
	| AppInputSpec<'datetime', string>
	| AppInputSpec<'object', Record<string | number, any>>
	| AppInputSpec<'array', any[]>
	| (AppInputSpec<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })

export type AppInputs = Record<string, AppInput>

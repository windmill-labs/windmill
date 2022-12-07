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
export type ResultInput<T, U> = {
	runnable: Runnable
	fields: Record<string, RunnableField<T, U>> | undefined
	type: 'runnable'
}

type ComponentInputSpec<T, U> = (StaticInput<U> | ConnectedInput | ResultInput<T, U>) &
	InputConfiguration<T, U>

type RunnableField<T, U> = (StaticInput<U> | ConnectedInput | UserInput<U>) &
	InputConfiguration<T, U>

// For example, the configuration of a button; size, color, etc.
type ComponentParameterSpec<T, U> = (StaticInput<U> | ConnectedInput) & InputConfiguration<T, U>

type InputConfiguration<T, U> = {
	fieldType: T
	defaultValue: U
}

export type ConnectableInput =
	| RunnableField<'text', string>
	| RunnableField<'textarea', string>
	| RunnableField<'number', number>
	| RunnableField<'boolean', boolean>
	| RunnableField<'date', string>
	| RunnableField<'time', string>
	| RunnableField<'datetime', string>
	| RunnableField<'object', Record<string | number, any>>
	| RunnableField<'array', Array<any>>
	| (RunnableField<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })

export type ComponentInput =
	| ComponentInputSpec<'text', string>
	| ComponentInputSpec<'textarea', string>
	| ComponentInputSpec<'number', number>
	| ComponentInputSpec<'boolean', boolean>
	| ComponentInputSpec<'date', string>
	| ComponentInputSpec<'time', string>
	| ComponentInputSpec<'datetime', string>
	| ComponentInputSpec<'object', Record<string | number, any>>
	| ComponentInputSpec<'array', Array<any>>
	| (ComponentInputSpec<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })

export type ComponentParameter =
	| ComponentParameterSpec<'text', string>
	| ComponentParameterSpec<'textarea', string>
	| ComponentParameterSpec<'number', number>
	| ComponentParameterSpec<'boolean', boolean>
	| ComponentParameterSpec<'date', string>
	| ComponentParameterSpec<'time', string>
	| ComponentParameterSpec<'datetime', string>
	| ComponentParameterSpec<'object', Record<string | number, any>>
	| ComponentParameterSpec<'array', Array<any>>
	| (ComponentParameterSpec<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })

export type InputsSpec = Record<string, ComponentInput>

export type ComponentConfiguration = Record<string, ComponentParameter>

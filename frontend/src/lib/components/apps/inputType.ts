import type { staticValues } from './editor/componentsPanel/componentStaticValues'
import type { InlineScript } from './types'

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
	| 'array'
	| 'any'

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
	runType: 'script' | 'flow' | 'hubscript'
	type: 'runnableByPath'
}

type RunnableByName = {
	name: string
	inlineScript: InlineScript | undefined
	type: 'runnableByName'
}

export type Runnable = RunnableByPath | RunnableByName | undefined

// Runnable input, set by the developer in the component panel
export type ResultInput = {
	runnable: Runnable
	fields: Record<string, StaticAppInput | ConnectedAppInput>
	type: 'runnable'
}

type AppInputSpec<T extends InputType, U, V extends InputType = never> = (
	| StaticInput<U>
	| ConnectedInput
	| UserInput<U>
	| ResultInput
) &
	InputConfiguration<T, U, V>

type InputConfiguration<T extends InputType, U, V extends InputType> = {
	fieldType: T
	defaultValue: U
	subFieldType?: V
}

export type AppInput =
	| AppInputSpec<'text', string>
	| AppInputSpec<'textarea', string>
	| AppInputSpec<'number', number>
	| AppInputSpec<'boolean', boolean>
	| AppInputSpec<'date', string>
	| AppInputSpec<'time', string>
	| AppInputSpec<'datetime', string>
	| AppInputSpec<'any', any>
	| AppInputSpec<'object', Record<string | number, any>>
	| (AppInputSpec<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })
	| AppInputSpec<'array', string[], 'text'>
	| AppInputSpec<'array', string[], 'textarea'>
	| AppInputSpec<'array', number[], 'number'>
	| AppInputSpec<'array', boolean[], 'boolean'>
	| AppInputSpec<'array', string[], 'date'>
	| AppInputSpec<'array', string[], 'time'>
	| AppInputSpec<'array', string[], 'datetime'>
	| AppInputSpec<'array', object[], 'object'>
	| (AppInputSpec<'array', string[], 'select'> & {
			optionValuesKey: keyof typeof staticValues
	  })

export type StaticAppInput = Extract<AppInput, { type: 'static' }>
export type ConnectedAppInput = Extract<AppInput, { type: 'connected' }>
export type UserAppInput = Extract<AppInput, { type: 'user' }>
export type ResultAppInput = Extract<AppInput, { type: 'runnable' }>

export type AppInputs = Record<string, AppInput>

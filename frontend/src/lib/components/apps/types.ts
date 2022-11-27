import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { FilledItem } from 'svelte-grid'
import type { Writable } from 'svelte/store'
import type { staticValues } from './editor/componentsPanel/componentStaticValues'
import type { World } from './rx'

export type UserInput<T, V> = {
	type: 'user'
	value: V | undefined
	defaultValue: V
	fieldType: T
}

export type DynamicInput<T, V> = {
	type: 'output'
	id: FieldID | undefined
	name: string | undefined
	defaultValue: V
	fieldType: T
}

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

export type StaticInput<T, V> = {
	value: V | undefined
	type: 'static'
	visible?: boolean
	defaultValue: V
	fieldType: T
}

type AppInput<T extends InputType, V> = StaticInput<T, V> | DynamicInput<T, V> | UserInput<T, V>

export type AppInputTransform =
	| AppInput<'text', string>
	| AppInput<'textarea', string>
	| AppInput<'number', number>
	| AppInput<'boolean', boolean>
	| (AppInput<'select', string> & {
			/**
			 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
			 */
			optionValuesKey: keyof typeof staticValues
	  })
	| AppInput<'date', string>
	| AppInput<'time', string>
	| AppInput<'datetime', string>
	| AppInput<'object', Record<string | number, any>>

// Inner inputs, (search, filter, page, inputs of a script or flow)
export type InputsSpec = Record<FieldID, AppInputTransform>

type Runnable = {
	inlineScriptName?: string
	path?: string
	runType?: 'script' | 'flow'
}

type BaseComponent<T extends string> = {
	type: T
}

export type TextComponent = BaseComponent<'textcomponent'>
export type TextInputComponent = BaseComponent<'textinputcomponent'>
export type ButtonComponent = Runnable & BaseComponent<'buttoncomponent'>
export type RunFormComponent = Runnable & BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = Runnable & BaseComponent<'piechartcomponent'>
export type TableComponent = Runnable &
	BaseComponent<'tablecomponent'> & {
		components: AppComponent[]
	}

export type DisplayComponent = BaseComponent<'displaycomponent'>
export type ImageComponent = BaseComponent<'imagecomponent'>
export type InputComponent = BaseComponent<'inputcomponent'>
export type SelectComponent = BaseComponent<'selectcomponent'>
export type CheckboxComponent = BaseComponent<'checkboxcomponent'>
export type RadioComponent = BaseComponent<'radiocomponent'>

export type HorizontalAlignment = 'left' | 'center' | 'right'
export type VerticalAlignment = 'top' | 'center' | 'bottom'
export type Aligned = {
	horizontalAlignment: HorizontalAlignment
	verticalAlignment: VerticalAlignment
}

export interface BaseAppComponent extends Partial<Aligned> {
	id: ComponentID
	inputs: InputsSpec
	componentInputs: InputsSpec
	runnable?: boolean | undefined
	card?: boolean | undefined

	// TODO: add min/max width/height
}

export type AppComponent = BaseAppComponent &
	(
		| RunFormComponent
		| DisplayComponent
		| TextInputComponent
		| BarChartComponent
		| TableComponent
		| TextComponent
		| TableComponent
		| ButtonComponent
		| PieChartComponent
		| ImageComponent
		| InputComponent
		| SelectComponent
		| CheckboxComponent
		| RadioComponent
	)

export type ComponentSet = {
	title: string
	components: AppComponent[]
}

type SectionID = string

export type AppSection = {
	components: AppComponent[]
	id: SectionID
}

export type GridItem = FilledItem<{
	data: AppComponent
	id: string
}>

export type App = {
	grid: GridItem[]
	inlineScripts: Record<
		string,
		{ content: string; language: Preview.language; path: string; schema: Schema }
	>
	title: string
}

export type ConnectingInput<T, V> = {
	opened: boolean
	input?: DynamicInput<T, V>
}

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selectedComponent: Writable<string | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput<any, any>>
}

export type EditorMode = 'dnd' | 'preview'

type FieldID = string

type ComponentID = string

export type EditorConfig = {
	staticInputDisabled: boolean
	outputInputDisabled: boolean
	userInputEnabled: boolean
	visibiltyEnabled: boolean
}

import type { Schema, SchemaProperty } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { FilledItem } from 'svelte-grid'
import type { Writable } from 'svelte/store'
import type { staticValues } from './editor/componentsPanel/componentStaticValues'
import type { World } from './rx'

export type UserInput = {
	type: 'user'
	schemaProperty: SchemaProperty
	// Default value override
	defaultValue: any
	value: any
}

export type DynamicInput = {
	type: 'output'
	id: FieldID | undefined
	name: string | undefined
	// Before any connection occures
	defaultValue: any
}

export type StaticInputType =
	('text'
	| 'textarea'
	| 'number'
	| 'boolean'
	| 'select'
	| 'date'
	| 'time'
	| 'datetime'
	| 'object')

type BaseStaticInput<T extends StaticInputType, V> = {
	fieldType: T
	value: V
	type: 'static'
	visible?: boolean
}

export type SelectStaticInput = BaseStaticInput<'select', string> & {
	/**
	 * One of the keys of `staticValues` from `lib/components/apps/editor/componentsPanel/componentStaticValues`
	 */
	optionValuesKey: keyof typeof staticValues
}

export type StaticInput = 
		BaseStaticInput<'text', string>
	| BaseStaticInput<'textarea', string>
	| BaseStaticInput<'number', number>
	| BaseStaticInput<'boolean', boolean>
	| SelectStaticInput
	| BaseStaticInput<'date', string>
	| BaseStaticInput<'time', string>
	| BaseStaticInput<'datetime', string>
	| BaseStaticInput<'object', Record<string | number, any>>


export type AppInputTransform = DynamicInput | StaticInput | UserInput

// Inner inputs, (search, filter, page, inputs of a script or flow)
export type InputsSpec = Record<FieldID, AppInputTransform>
export type ComponentInputsSpec = Record<FieldID, DynamicInput | StaticInput>

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
export type ButtonComponent = BaseComponent<'buttoncomponent'>
export type RunFormComponent = Runnable & BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = Runnable & BaseComponent<'piechartcomponent'>
export type TableComponent = Runnable & BaseComponent<'tablecomponent'>
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
	// Only dynamic inputs (Result of display)
	componentInputs: ComponentInputsSpec
	runnable?: boolean | undefined
	card?: boolean | undefined

	// TODO: add min/max width/height
}

export type AppComponent =
	BaseAppComponent & (
		RunFormComponent
		| DisplayComponent
		| TextInputComponent
		| BarChartComponent
		| TableComponent
		| TextComponent
		| TableComponent
		| ButtonComponent
		| PieChartComponent
		|	ImageComponent
		|	InputComponent
		|	SelectComponent
		|	CheckboxComponent
		|	RadioComponent
	)

export type ComponentSet = {
	title: string,
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

export type ConnectingInput = {
	opened: boolean
	input?: DynamicInput
}

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selectedComponent: Writable<string | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput>
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

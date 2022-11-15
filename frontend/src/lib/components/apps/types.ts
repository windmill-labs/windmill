import type { Schema, SchemaProperty } from '$lib/common'
import type { Policy } from '$lib/gen'
import type { Writable } from 'svelte/store'
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

export type StaticInput = {
	type: 'static'
	value: any
	visible?: boolean
}

export type AppInputTransform = DynamicInput | StaticInput | UserInput

// Inner inputs, (search, filter, page, inputs of a script or flow)
export type InputsSpec = Record<FieldID, AppInputTransform>
export type ComponentInputsSpec = Record<FieldID, DynamicInput>

export type TextComponent = {
	type: 'textcomponent'
}

export type TextInputComponent = {
	type: 'textinputcomponent'
}

export type RunFormComponent = {
	type: 'runformcomponent'
	path?: string
	runType?: 'script' | 'flow'
}

export type BarChartComponent = {
	type: 'barchartcomponent'
	inputs: {}
}

export type TableComponent = {
	type: 'tablecomponent'
	inputs: {}
	path: string
	runType: 'script' | 'flow'
	title: string
	description: string | undefined
	headers: string[]
	data: Array<Record<string, any>>
}

export type DisplayComponent = {
	type: 'displaycomponent'
}

export type AppComponent =
	| (
			| RunFormComponent
			| DisplayComponent
			| TextInputComponent
			| BarChartComponent
			| TableComponent
			| TextComponent
	  ) & {
			id: ComponentID
			title: string
			description: string
			width: number
			horizontalAlignement?: 'left' | 'center' | 'right'
			verticalAlignement?: 'top' | 'center' | 'bottom'
			configSchema: Schema | undefined
			inputs: InputsSpec
			componentInputs: ComponentInputsSpec
	  }

type SectionID = string

export type AppSection = {
	title: string
	description: string
	components: AppComponent[]
	id: SectionID
	columns: 1 | 2 | 3
}

export type App = {
	sections: AppSection[]
	title: string
}

export type AppSelection = { sectionIndex: number; componentIndex: number | undefined }

export type ConnectingInput = {
	opened: boolean
	input?: DynamicInput
}

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selection: Writable<AppSelection | undefined>
	mode: Writable<EditorMode>
	schemas: Writable<Schema[]>
	connectingInput: Writable<ConnectingInput>
	resizing: Writable<boolean>
}

export type EditorMode = 'width' | 'dnd' | 'preview'

type FieldID = string

type ComponentID = string

export type EditorConfig = {
	staticInputDisabled: boolean
	outputInputDisabled: boolean
	userInputEnabled: boolean
	visibiltyEnabled: boolean
}

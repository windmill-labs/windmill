import type { Schema, SchemaProperty } from '$lib/common'
import type { Writable } from 'svelte/store'
import type { World } from './rx'

export type UserInput = {
	type: 'user'
	schemaProperty: SchemaProperty
	// Default value override
	defaultValue: any
}

export type DynamicInput = {
	type: 'output'
	id: FieldID
	name: string
	visible?: boolean //default false
}

export type StaticInput = {
	type: 'static'
	visible?: boolean //default false
}

export type AppInputTransform = DynamicInput | StaticInput | UserInput

// From ID of component + ID of field -> retrieve value from Policy
export type InputsSpec = Record<FieldID, AppInputTransform>

export type TextInputComponent = {
	type: 'textinputcomponent'
}

export type RunFormComponent = {
	type: 'runformcomponent'
	inputs: InputsSpec
	params: {
		hidden: string[]
	}
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
	inputs: InputsSpec
}

export type AppComponent =
	| (
			| RunFormComponent
			| DisplayComponent
			| TextInputComponent
			| BarChartComponent
			| TableComponent
	  ) & {
			id: ComponentID
			title: string
			description: string
			width: number
			horizontalAlignement?: 'left' | 'center' | 'right'
			verticalAlignement?: 'top' | 'center' | 'bottom'
			configSchema: Schema | undefined
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
	policy: Policy | undefined
}

export type AppSelection = { sectionIndex: number; componentIndex: number | undefined }

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selection: Writable<AppSelection | undefined>
	mode: Writable<EditorMode>
	schemas: Writable<Schema[]>
}

export type EditorMode = 'width' | 'dnd' | 'preview'

type FieldID = string

export interface TriggerablePolicy {
	path: string
	staticFields: Record<FieldID, any>
	type: 'script' | 'flow'
}

interface Policy {
	triggerables: Record<ComponentID, TriggerablePolicy>
}

type ComponentID = string

interface Policy {
	triggerables: Record<ComponentID, TriggerablePolicy>
}

enum PublishedStatus {
	ViewerPerms = 'ViewerPerms',
	AuthorPermsUser = 'AuthorPermsUser',
	AuthorPermsPublic = 'AuthorPermsPublic'
}

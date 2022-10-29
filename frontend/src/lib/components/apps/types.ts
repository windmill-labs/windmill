import type { Writable } from 'svelte/store'
import type { World } from './rx'

export type DynamicInput = {
	type: 'output'
	id: string
	name: string
}

export type StaticInput = {
	type: 'static'
	value: any
}

export type AppInputTransform = DynamicInput | StaticInput

export type InputsSpec = Record<string, AppInputTransform>

export type TextInputComponent = {
	type: 'textinputcomponent'
}

export type RunFormComponent = {
	runType: 'script' | 'flow'
	path: string
	type: 'runformcomponent'
	inputs: {
		runInputs: InputsSpec
	}
	params: {
		hidden: string[]
	}
}

export type DisplayComponent = {
	type: 'displaycomponent'
	inputs: {
		result: AppInputTransform
	}
}

export type AppComponent =
	| ((RunFormComponent | DisplayComponent | TextInputComponent) & {
			id: string
	  })
	| undefined

export type AppSection = {
	components: AppComponent[]
	columns: number
}

export type App = {
	sections: AppSection[]
	title: string
}

export type AppSelection = { sectionIndex: number; componentIndex: number | undefined }

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selection: Writable<AppSelection | undefined>
}

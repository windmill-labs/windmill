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
		result: InputsSpec
	}
}

export type AppComponent = (RunFormComponent | DisplayComponent | TextInputComponent) & {
	id: string
}

export type App = {
	components: AppComponent[]
	title: string
}

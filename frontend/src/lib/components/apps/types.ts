
type AppInputTransform = {
	type: 'output'
	id: string
	name: string
} | {
	type: 'static'
	value: any
}

type InputsLeaf = Record<string, AppInputTransform>
type Inputs2 = Record<string, AppInputTransform | InputsLeaf>
type Inputs1 = Record<string, AppInputTransform | Inputs2>
type Inputs = Record<string, AppInputTransform | Inputs1>

export type TextInputComponent = {
	type: 'textinputcomponent'
}

export type RunFormComponent = {
	runType: 'script' | 'flow'
	path: string
	type: 'runformcomponent'
	inputs: {
		runInputs: Inputs
	}
}

export type DisplayComponent = {
	type: 'displaycomponent'
	inputs: {
		result: Inputs
	}
}

export type AppComponent = (RunFormComponent | DisplayComponent | TextInputComponent) & {
	id: string
}

export type App = {
	components: AppComponent[]
	title: string
}

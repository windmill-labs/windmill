import type { InputTransform } from '$lib/gen'

interface ComponentInputs {
	inputs: Record<string, InputTransform>
}

type InputsLeaf = Record<string, InputTransform>
type Inputs2 = Record<string, InputTransform | InputsLeaf>
type Inputs1 = Record<string, InputTransform | Inputs2>
type Inputs = Record<string, InputTransform | Inputs1>

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

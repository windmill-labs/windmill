import { type InputTransform } from '$lib/gen'
import type { Writable } from 'svelte/store'

export type FlowCopilotContext = {
	currentStepStore: Writable<string | undefined>
	shouldUpdatePropertyType: Writable<{
		[key: string]: 'static' | 'javascript' | undefined
	}>
	stepInputsLoading: Writable<boolean>
	generatedExprs: Writable<{
		[key: string]: string | undefined
	}>
	exprsToSet: Writable<{
		[key: string]: InputTransform | undefined
	}>
}

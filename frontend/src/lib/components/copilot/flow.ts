import { type InputTransform } from '$lib/gen'
import type { Writable } from 'svelte/store'

export type FlowCopilotContext = {
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
	toggleAiPanel?: () => void
	addSelectedLinesToAiChat?: (lines: string, startLine: number, endLine: number) => void
	fix?: () => void
}

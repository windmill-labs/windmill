import { tweened } from 'svelte/motion'
import { cubicOut } from 'svelte/easing'

export interface ProgressStateStoreValue {
	length: number
	index: number
	finished: boolean
	error: boolean
}

export type StepKind = 'general' | 'loop'

type Step<T extends StepKind> = {
	type: T
	isDone: boolean
}

export type GeneralStep = Step<'general'>
export type LoopStep = Step<'loop'> & { index: number; length: number }
export type ProgressStep = GeneralStep | LoopStep

export function isLoopStep(step: ProgressStep | undefined): step is LoopStep {
	return step?.type === 'loop'
}

type State<T extends StepKind> = {
	type: T
	isDone: boolean
	isDoneChanged: boolean
}

export type GeneralState = State<'general'>
export type LoopState = LoopStep & { index: number; indexChanged: boolean }
export type ProgressState = GeneralState | LoopState

export function isLoopState(state: ProgressState | undefined): state is LoopState {
	return state?.type === 'loop'
}

export function getTween(initialValue = 0, duration = 200) {
	return tweened(initialValue, {
		duration,
		easing: cubicOut
	})
}

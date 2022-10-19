export enum StepKind {
	script = 'Script',
	loopStep = 'Loop Step'
}

export const TYPE_STEP_MAPPING = {
	script: StepKind.script,
	rawscript: StepKind.script,
	flow: StepKind.script,
	forloopflow: StepKind.loopStep
} as const

export type ProgressStep = StepKind.script | StepKind.loopStep[]

export type Progress = ProgressStep[]

export function isLoop(step: ProgressStep): step is StepKind.loopStep[] {
	return Array.isArray(step)
}

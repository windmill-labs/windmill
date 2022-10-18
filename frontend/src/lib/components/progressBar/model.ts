export enum StepKind {
	script = 'Script',
	loopStep = 'Loop Step'
}

export type ProgressStep = StepKind.script | StepKind.loopStep[]

export type Progress = ProgressStep[]

export function isLoop(step: ProgressStep): step is StepKind.loopStep[] {
	return Array.isArray(step)
}

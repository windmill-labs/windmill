import type { Flow, FlowModule, Job } from '$lib/gen'
import { buildExtraLib, objectToTsType, schemaToObject, schemaToTsType } from '$lib/utils'
import { get } from 'svelte/store'
import { flowStateStore, type FlowModuleState, type FlowState } from './flowState'
import { NEVER_TESTED_THIS_FAR } from './utils'

type Result = any

type PickableProperties = {
	flow_input?: Object
	previous_result: Result | undefined
	step?: Result[]
}

type StepPropPicker = {
	pickableProperties: PickableProperties
	extraLib: string
}

function dfs(id: string, flow: Flow): FlowModule[] {
	function getSubModules(flowModule: FlowModule): FlowModule[] {
		if (flowModule.value.type === 'forloopflow') {
			return flowModule.value.modules
		} else if (flowModule.value.type === 'branchall') {
			return flowModule.value.branches.map((branch) => branch.modules).flat()
		} else if (flowModule.value.type == 'branchone') {
			return [
				...flowModule.value.branches.map((branch) => branch.modules).flat(),
				...flowModule.value.default
			]
		}
		return []
	}

	function rec(id: string, modules: FlowModule[]): FlowModule[] | undefined {
		for (let module of modules) {
			if (module.id === id) {
				return [module]
			} else {
				const submodules = getSubModules(module)

				if (submodules) {
					let found: FlowModule[] | undefined = undefined
					found = rec(id, submodules)

					if (found) {
						break
					}

					if (module && found) {
						return [...found, module]
					} else {
						return undefined
					}
				} else {
					return undefined
				}
			}
		}
	}

	return rec(id, flow.value.modules) ?? []
}

function flattenPreviousResult(pr: any) {
	if (typeof pr === 'object' && pr.previous_result) {
		return pr.previous_result
	}

	return pr
}

function getFlowInput(
	parentModule: FlowModule | undefined,
	flowState: FlowState,
	args: any,
	flow: Flow,
	grandParentModules: FlowModule[] | undefined = undefined
) {
	const parentState = parentModule ? flowState[parentModule.id] : undefined

	if (parentState && parentModule) {
		if (parentState.previewArgs) {
			return parentState.previewArgs
		} else {
			const gpm: FlowModule[] = grandParentModules ?? dfs(parentModule.id, flow)
			const head = gpm.pop()
			const parentFlowInput = getFlowInput(head, flowState, args, flow, gpm)

			if (parentModule.value.type === 'forloopflow') {
				return {
					...parentFlowInput,
					iter: {
						value: "Iteration's value",
						index: "Iteration's index"
					}
				}
			} else {
				// Branches

				return {
					...parentFlowInput
					// TODO: Fix previous_result: flattenPreviousResult(parentFlowInput)
				}
			}
		}
	} else {
		return schemaToObject(flow.schema, args)
	}
}

function getPriorIds(flow: Flow, id: string): string[] {
	// TODO: Ruben
	return flow.value.modules.map((module) => module.id)
}

export function getStepPropPicker(
	flowState: FlowState,
	parentModule: FlowModule | undefined,
	previousModuleId: string | undefined,
	flow: Flow,
	args: any
): StepPropPicker {
	const flowInput = getFlowInput(parentModule, flowState, args, flow)
	const previousResults = previousModuleId
		? flowStateStore[previousModuleId].previewResult
		: flattenPreviousResult(flowInput)
	//const priorIds = getPriorIds(flow, parentModule.id)

	return {
		extraLib: buildExtraLib(objectToTsType(flowInput), objectToTsType(previousResults)),
		pickableProperties: {
			flow_input: flowInput,
			previous_result: previousResults
		}
	}
}

// OTHER FILE

export type JobResult = {
	job?: Job
	innerJobs: JobResult[]
	loopJobs?: JobResult[]
}

function getResult(job: Job | undefined): Result | undefined {
	if (job && 'result' in job) {
		return job.result
	}
}

export function mapJobResultsToFlowState(jobs: JobResult, upto: number): void {
	const results = jobs.innerJobs.map(({ job, loopJobs }) => {
		if (loopJobs && loopJobs.length > 0) {
			return [
				job?.args,
				loopJobs.map(({ job }) => {
					return getResult(job)
				})
			]
		} else {
			return [job?.args, getResult(job)]
		}
	})

	const old = get(flowStateStore)

	/*
	const modules = old.modules.map((flowModuleState: FlowModuleState, index: number) => {
		if (results[index] && index <= upto) {
			if (
				results[index][1] != NEVER_TESTED_THIS_FAR ||
				flowModuleState.previewResult == undefined
			) {
				flowModuleState.previewArgs = results[index][0]
				flowModuleState.previewResult = results[index][1]
				flowModuleState.childFlowModules?.map((innerMod, j) => {
					const lastLoopJob = jobs.innerJobs[index].loopJobs?.length ?? 0
					innerMod.previewResult = getResult(
						jobs.innerJobs[index].loopJobs?.[lastLoopJob - 1]?.innerJobs?.[j]?.job
					)
				})
			}
		}

		return flowModuleState
	})
*/
	flowStateStore.set({
		modules: old.modules,
		failureModule: old.failureModule
	})
}

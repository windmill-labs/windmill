import type { Schema } from '$lib/common'
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

function getPreviousResults(): any {
	/**
	 * 1. if previous, previous.result
	 * 	 else flattenPreviousResult(getFlowInput())
	 */
}

function dfs(id: string, flow: Flow): FlowModule[] {
	function getSubModules(flowModule: FlowModule): FlowModule[] | undefined {
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
	}

	function rec(
		flowModule: FlowModule | undefined,
		flow: Flow | undefined = undefined
	): FlowModule[] | undefined {
		if (flowModule?.id === id) {
			return [flowModule]
		} else {
			const submodules = flow ? flow.value.modules : getSubModules(flowModule)

			if (submodules) {
				let found: FlowModule[] | undefined = undefined

				for (let submodule of submodules) {
					found = rec(submodule)

					if (found) {
						break
					}
				}

				if (flowModule && found) {
					return [...found, flowModule]
				} else {
					return undefined
				}
			} else {
				return undefined
			}
		}
	}

	return rec(undefined, flow) ?? []
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
	flowInputSchema: Schema,
	args,
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
			const parentFlowInput = getFlowInput(head, flowState, flowInputSchema, args, flow, gpm)

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
					...parentFlowInput,
					previous_result: flattenPreviousResult(getPreviousResults())
				}
			}
		}
	} else {
		return schemaToObject(flowInputSchema, args)
	}
}

function getPriorIds(flow: Flow, id: string): string[] {
	// TODO: Ruben
	return flow.value.modules.map((module) => module.id)
}

export function getStepPropPicker(
	flowState: FlowState,
	parentModule: FlowModule,
	flow: Flow,
	args: any,
	flowInputSchema: Schema
): StepPropPicker {
	const flowInput = getFlowInput(parentModule, flowState, flowInputSchema, flow, args)
	const previousResults = {} //getPreviousResults()
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

	flowStateStore.set({
		modules,
		failureModule: old.failureModule
	})
}

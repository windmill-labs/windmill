import type { Schema } from '$lib/common'
import type { Job } from '$lib/gen'
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

export function getStepPropPicker(
	id: string,
	flowInputSchema: Schema,
	flowState: FlowState,
	flowModuleMap: any,
	args: Record<string, any>
): StepPropPicker {
	return {
		pickableProperties: {
			previous_result: undefined
		},
		extraLib: ''
	}

	//const isInsideLoop: boolean = childIndex !== undefined

	/*
	const { flowModule, parentModuleId, previousModuleId } = flowModuleMap[id]
	const flowInput = schemaToObject(flowInputSchema, args)

	if (parenteModuleId) {
	} else {
		const extraLib = buildExtraLib(schemaToTsType(flowInputSchema), objectToTsType(lastResult))

		return {
			extraLib,
			pickableProperties: {
				flow_input: flowInput,
				previous_result: lastResult,
				step: results
			}
		}
	}

	/*

	const flowInput = schemaToObject(flowInputSchema, args)
	const results = getPreviousResults(flowState.modules, parentIndex)

	const lastResult =
		parentIndex == 0
			? flowInput
			: results.length > 0
			? results[results.length - 1]
			: NEVER_TESTED_THIS_FAR

	if (isInsideLoop) {
		let forLoopFlowInput = {
			...flowInput,
			iter: {
				value: "Iteration's value",
				index: "Iteration's index"
			}
		}

		if (flowState.modules[parentIndex]?.previewArgs) {
			forLoopFlowInput = flowState.modules[parentIndex]?.previewArgs
		}

		const innerResults = getPreviousResults(
			flowState.modules[parentIndex]?.childFlowModules,
			childIndex
		)

		const innerLastResult =
			childIndex == 0
				? forLoopFlowInput
				: innerResults.length > 0
				? innerResults[innerResults.length - 1]
				: NEVER_TESTED_THIS_FAR

		const extraLib = buildExtraLib(
			objectToTsType(forLoopFlowInput),
			objectToTsType(innerLastResult)
		)

		return {
			extraLib,
			pickableProperties: {
				flow_input: forLoopFlowInput,
				previous_result: innerLastResult,
				step: innerResults
			}
		}
	} else {
		const extraLib = buildExtraLib(schemaToTsType(flowInputSchema), objectToTsType(lastResult))

		return {
			extraLib,
			pickableProperties: {
				flow_input: flowInput,
				previous_result: lastResult,
				step: results
			}
		}
	}
	*/
}

function getPreviousResults(
	flowModuleSchemas: FlowModuleState[] | undefined,
	target: number
): Result[] {
	if (!Array.isArray(flowModuleSchemas) || target < 1) {
		return []
	}

	const results = extractPreviewResults(flowModuleSchemas)
	return results.splice(0, target)
}

function extractPreviewResults(flowModuleSchemas: FlowModuleState[]) {
	return flowModuleSchemas.map((fms) => fms.previewResult)
}

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

import type { Schema } from '$lib/common'
import { CompletedJob, Job, Script, ScriptService, type FlowModule, type RawScript } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import {
	buildExtraLib,
	emptySchema,
	getScriptByPath,
	objectToTsType,
	schemaToObject,
	schemaToTsType
} from '$lib/utils'
import { get } from 'svelte/store'
import { flowStateStore, type FlowModuleState, type FlowState } from './flowState'
import { flowStore } from './flowStore'
import { loadSchemaFromModule } from './utils'

export function emptyFlowModuleState(): FlowModuleState {
	return {
		schema: emptySchema()
	}
}

export async function loadFlowModuleSchema(flowModule: FlowModule): Promise<FlowModuleState> {
	try {
		const { input_transforms, schema } = await loadSchemaFromModule(flowModule)

		flowModule.input_transforms = input_transforms

		return { schema, previewResult: NEVER_TESTED_THIS_FAR }
	} catch (e) {
		return { schema: emptySchema(), previewResult: NEVER_TESTED_THIS_FAR }
	}
}

export async function pickScript(path: string): Promise<[FlowModule, FlowModuleState]> {
	const flowModule: FlowModule = {
		value: { type: 'script', path },
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleSchema(flowModule)]
}

export async function createInlineScriptModule({
	language,
	kind,
	subkind
}: {
	language: RawScript.language
	kind: Script.kind
	subkind: 'pgsql' | 'flow'
}): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		value: { type: 'rawscript', content: code, language },
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleSchema(flowModule)]
}

export async function createLoop(): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		value: {
			type: 'forloopflow',
			modules: [],
			iterator: { type: 'javascript', expr: 'previous_result' },
			skip_failures: true
		},
		input_transforms: {}
	}

	const { schema } = await loadFlowModuleSchema(loopFlowModule)
	return [
		loopFlowModule,
		{
			schema,
			childFlowModules: [emptyFlowModuleState()],
			previewResult: NEVER_TESTED_THIS_FAR
		}
	]
}

export async function fork(flowModule: FlowModule): Promise<[FlowModule, FlowModuleState]> {
	if (flowModule.value.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
	const fm = await createInlineScriptModuleFromPath(flowModule.value.path ?? '')

	return [fm, await loadFlowModuleSchema(fm)]
}

export async function createInlineScriptModuleFromPath(path: string): Promise<FlowModule> {
	const { content, language } = await getScriptByPath(path)

	return {
		value: {
			type: 'rawscript',
			language: language as RawScript.language,
			content: content,
			path
		},
		input_transforms: {}
	}
}

export async function createScriptFromInlineScript({
	flowModule,
	suffix,
	schema
}: {
	flowModule: FlowModule
	suffix: string
	schema: Schema
}): Promise<[FlowModule, FlowModuleState]> {
	const flow = get(flowStore)
	const user = get(userStore)

	if (flowModule.value.type != 'rawscript') {
		throw new Error("Can't create script from non-inline script")
	}

	const originalScriptPath = flowModule.value.path
	const wasForked = Boolean(originalScriptPath)

	if (wasForked && originalScriptPath) {
		const [first, second, ...others] = originalScriptPath.split('/')
		suffix = others.join('/')
	}

	const path = `${flow.path}/${suffix}`
	const forkedDescription = wasForked ? `as a fork of ${originalScriptPath}` : ''
	const description = `This script was edited in place of flow ${flow.path} ${forkedDescription} by ${user?.username}}.`

	const availablePath = await findNextAvailablePath(path)

	await ScriptService.createScript({
		workspace: get(workspaceStore)!,
		requestBody: {
			path: availablePath,
			summary: '',
			description,
			content: flowModule.value.content,
			parent_hash: undefined,
			schema,
			is_template: false,
			language: flowModule.value.language
		}
	})

	return pickScript(availablePath)
}

async function findNextAvailablePath(path: string): Promise<string> {
	try {
		await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path
		})

		const [_, version] = path.split(/.*_([0-9]*)/)

		if (version.length > 0) {
			path = path.slice(0, -(version.length + 1))
		}

		path = `${path}_${Number(version) + 1}`

		return findNextAvailablePath(path)
	} catch (e) {
		// Catching an error means the path is available
		return path
	}
}

export function isEmptyFlowModule(flowModule: FlowModule): boolean {
	return flowModule.value.type === 'script' && flowModule.value.path === ''
}

type Result = any

type PickableProperties = {
	flow_input?: Object
	previous_result: Result | undefined
	step: Result[]
}

type StepPropPicker = {
	pickableProperties: PickableProperties
	extraLib: string
}

export const NEVER_TESTED_THIS_FAR = 'never tested this far'

export function getStepPropPicker(
	indexes: number[],
	flowInputSchema: Schema,
	flowState: FlowState,
	args: Record<string, any>
): StepPropPicker {
	const isInsideLoop: boolean = indexes.length > 1
	const [parentIndex, childIndex] = indexes

	const flowInput = schemaToObject(flowInputSchema, args)
	const results = getPreviousResults(flowState.modules, parentIndex)

	const lastResult =
		parentIndex == 0
			? flowInput
			: results.length > 0
			? results[results.length - 1]
			: NEVER_TESTED_THIS_FAR

	if (isInsideLoop) {
		const forLoopFlowInput = {
			...flowInput,
			iter: {
				value: "Iteration's value",
				index: "Iteration's index"
			}
		}

		if (Array.isArray(lastResult) && lastResult.length > 0) {
			const last = lastResult[lastResult.length - 1]

			forLoopFlowInput.iter = {
				value: last,
				index: `Iteration's index (0 to ${lastResult.length - 1})`
			}
		}

		const innerResults = getPreviousResults(
			flowState.modules[parentIndex].childFlowModules,
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

export function mapJobResultsToFlowState(
	jobs: JobResult,
	config: 'upto' | 'justthis',
	parentIndex: number,
	j: number | undefined
): void {
	if (config === 'justthis') {
		const job = jobs.job as CompletedJob

		flowStateStore.update((flowState: FlowState) => {
			if (flowState.modules) {
				const childFlowModules = flowState.modules[parentIndex].childFlowModules
				if (j && childFlowModules) {
					childFlowModules[j].previewResult = job.result
					flowState.modules[parentIndex].childFlowModules = childFlowModules
				} else {
					flowState.modules[parentIndex].previewResult = job.result
				}
			}

			return flowState
		})
	} else {
		if (jobs.innerJobs.length === 0) {
			return
		}

		const results = jobs.innerJobs.map(({ job, loopJobs }) => {
			if (Array.isArray(loopJobs) && loopJobs.length > 0) {
				return loopJobs.map(({ job }) => {
					if (job && 'result' in job) {
						return job.result
					}
				})
			} else {
				if (job && 'result' in job) {
					return job.result
				}
			}
		})

		flowStateStore.update((flowState: FlowState) => {
			if (!Array.isArray(flowState.modules)) {
				return flowState
			}

			const modules = flowState.modules.map((flowModuleState: FlowModuleState, index: number) => {
				if (index <= parentIndex) {
					flowModuleState.previewResult = results[index]
				}

				return flowModuleState
			})

			return {
				modules,
				failureModule: flowState.failureModule
			}
		})
	}
}

import type { Schema } from '$lib/common'
import { CompletedJob, ScriptService, type Flow, type FlowModule, type RawScript } from '$lib/gen'
import { initialCode } from '$lib/script_helpers'
import { userStore, workspaceStore } from '$lib/stores'
import {
	buildExtraLib,
	emptyModule,
	emptySchema,
	getScriptByPath,
	objectToTsType,
	schemaToObject,
	schemaToTsType
} from '$lib/utils'
import { get } from 'svelte/store'
import { flowStateStore, type FlowModuleSchema, type FlowState } from './flowState'
import { flowStore } from './flowStore'
import { jobsToResults, loadSchemaFromModule } from './utils'

export function emptyFlowModuleSchema(): FlowModuleSchema {
	return {
		flowModule: emptyModule(),
		schema: emptySchema()
	}
}

export async function loadFlowModuleSchema(flowModule: FlowModule): Promise<FlowModuleSchema> {
	try {
		const { input_transform, schema } = await loadSchemaFromModule(flowModule)

		flowModule.input_transform = input_transform

		return { flowModule, schema }
	} catch (e) {
		return { flowModule, schema: emptySchema() }
	}
}

export async function pickScript(path: string): Promise<FlowModuleSchema> {
	const flowModule: FlowModule = {
		value: { type: 'script', path },
		input_transform: {}
	}

	return await loadFlowModuleSchema(flowModule)
}

export async function createInlineScriptModule({
	language,
	isTrigger = false
}: {
	language: RawScript.language
	isTrigger: boolean
}): Promise<FlowModuleSchema> {
	const code = initialCode(language, isTrigger, true)

	const flowModule: FlowModule = {
		value: { type: 'rawscript', content: code, language },
		input_transform: {}
	}

	return await loadFlowModuleSchema(flowModule)
}

export async function createLoop(): Promise<FlowModuleSchema> {
	const loopFlowModule: FlowModule = {
		value: {
			type: 'forloopflow',
			value: {
				modules: []
			},
			iterator: { type: 'javascript', expr: 'result' },
			skip_failures: true
		},
		input_transform: {}
	}

	const { flowModule, schema } = await loadFlowModuleSchema(loopFlowModule)

	return {
		flowModule,
		schema,
		childFlowModules: [emptyFlowModuleSchema()],
		previewResult: undefined
	}
}

export async function fork(flowModule: FlowModule): Promise<FlowModuleSchema> {
	if (flowModule.value.type !== 'script') {
		throw new Error('Can only fork a script module')
	}
	const fm = await createInlineScriptModuleFromPath(flowModule.value.path ?? '')

	return await loadFlowModuleSchema(fm)
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
		input_transform: {}
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
}): Promise<FlowModuleSchema> {
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

export function getStepPropPicker(
	indexes: number[],
	flowInputSchema: Schema,
	flowState: FlowState,
	args: Record<string, any>
): StepPropPicker {
	const isInsideLoop: boolean = indexes.length > 1
	const [parentIndex, childIndex] = indexes

	const flowInput = schemaToObject(flowInputSchema, args)
	const results = getPreviousResults(flowState, parentIndex)
	const lastResult = results.length > 0 ? results[results.length - 1] : undefined

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

		/*
		const forLoopResults = getPreviousResults(flowState[parentIndex]?.childFlowModules, childIndex)
		const forLoopLastResult =
			forLoopResults.length > 0 ? forLoopResults[forLoopResults.length - 1] : undefined
		*/

		const extraLib = buildExtraLib(
			objectToTsType(forLoopFlowInput),
			//objectToTsType(forLoopLastResult)
			undefined
		)

		return {
			extraLib,
			pickableProperties: {
				flow_input: forLoopFlowInput,
				step: [],
				previous_result: {}
			}
		}
	} else {
		const extraLib = buildExtraLib(schemaToTsType(flowInputSchema), objectToTsType(lastResult))

		return {
			extraLib,
			pickableProperties: {
				flow_input: flowInput,
				step: results,
				previous_result: lastResult
			}
		}
	}
}

function getPreviousResults(
	flowModuleSchemas: FlowModuleSchema[] | undefined,
	target: number
): Result[] {
	if (!Array.isArray(flowModuleSchemas) || target < 1) {
		return []
	}

	const results = extractPreviewResults(flowModuleSchemas)
	return results.splice(0, target)
}

function extractPreviewResults(flowModuleSchemas: FlowModuleSchema[]) {
	return flowModuleSchemas.map((fms) => {
		if (fms.flowModule.value.type === 'forloopflow') {
			const { childFlowModules } = fms

			return Array.isArray(childFlowModules) ? childFlowModules.map((c) => c.previewResult) : []
		} else {
			return fms.previewResult
		}
	})
}

export function mapJobResultsToFlowState(
	jobs: CompletedJob[],
	config: 'upto' | 'justthis',
	configIndex: number
): void {
	if (!Array.isArray(jobs) || jobs.length === 0) {
		return
	}

	if (config === 'justthis') {
		const [result] = jobsToResults(jobs)

		flowStateStore.update((flowState: FlowState) => {
			flowState[configIndex] = result
			return flowState
		})
	} else {
		const result = jobsToResults(jobs)
		flowStateStore.update((flowState: FlowState) => {
			if (!Array.isArray(flowState)) {
				return flowState
			}

			return flowState.map((flowModuleSchema: FlowModuleSchema, index) => {
				if (index <= configIndex) {
					flowModuleSchema.previewResult = result[index]
				}

				return flowModuleSchema
			})
		})
	}
}

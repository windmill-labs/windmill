import type { Schema } from '$lib/common'
import { Job, Script, ScriptService, type FlowModule, type RawScript } from '$lib/gen'
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

		if (flowModule.value.type == 'script' || flowModule.value.type == 'rawscript') {
			flowModule.value.input_transforms = input_transforms
		}
		return { schema, previewResult: NEVER_TESTED_THIS_FAR }
	} catch (e) {
		return { schema: emptySchema(), previewResult: NEVER_TESTED_THIS_FAR }
	}
}

function computeLength(flowModuleStates: FlowModuleState[] | undefined) {
	let modules = flowModuleStates || []
	return modules.length + modules.map(x => computeLength(x.childFlowModules ?? [])).reduce(
		(a, b) => a + b,
		0
	);
}

const charCode = 'a'.charCodeAt(0);

function numberToChars(n: number) {

	var b = [n], sp, out, i, div;

	sp = 0;
	while (sp < b.length) {
		if (b[sp] > 25) {
			div = Math.floor(b[sp] / 26);
			b[sp + 1] = div - 1;
			b[sp] %= 26;
		}
		sp += 1;
	}

	out = "";
	for (i = 0; i < b.length; i += 1) {
		out = String.fromCharCode(charCode + b[i]) + out;
	}

	return out;
}


export function nextId(): string {
	const flowState = get(flowStateStore)
	const len = computeLength(flowState.modules)
	return numberToChars(len);
}
export async function pickScript(path: string): Promise<[FlowModule, FlowModuleState]> {
	const flowModule: FlowModule = {
		id: nextId(),
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
	subkind: 'pgsql' | 'flow',
}): Promise<[FlowModule, FlowModuleState]> {
	const code = initialCode(language, kind, subkind)

	const flowModule: FlowModule = {
		id: nextId(),
		value: { type: 'rawscript', content: code, language },
		input_transforms: {}
	}

	return [flowModule, await loadFlowModuleSchema(flowModule)]
}

export async function createLoop(): Promise<[FlowModule, FlowModuleState]> {
	const loopFlowModule: FlowModule = {
		id: nextId(),
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
		id: nextId(),
		value: {
			type: 'rawscript',
			language: language as RawScript.language,
			content: content,
			path
		},
		input_transforms: {}
	}
}

export function emptyModule(): FlowModule {
	return {
		id: nextId(),
		value: { type: 'script', path: '' },
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
	const description = `This script was edited in place of flow ${flow.path} ${forkedDescription} by ${user?.username}.`

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
	step?: Result[]
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
	const [parentIndex, childIndex] = indexes
	const isInsideLoop: boolean = childIndex !== undefined

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

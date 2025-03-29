import {
	JobService,
	type Flow,
	type FlowModule,
	type InputTransform,
	type Job,
	type RestartedFrom,
	type OpenFlow,
	type FlowModuleValue
} from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { cleanExpr, emptySchema } from '$lib/utils'
import { get } from 'svelte/store'
import type { FlowModuleState } from './flowState'
import { type PickableProperties, dfs } from './previousResults'
import { NEVER_TESTED_THIS_FAR } from './models'
import { sendUserToast } from '$lib/toast'
import type { Schema } from '$lib/common'
import { parseOutputs } from '$lib/infer'
import type { ExtendedOpenFlow } from './types'

function create_context_function_template(eval_string: string, context: Record<string, any>) {
	return `
return function (context) {
"use strict";
${
	Object.keys(context).length > 0
		? `let ${Object.keys(context).map((key) => ` ${key} = context['${key}']`)};`
		: ``
}
return ${eval_string}
}`
}

function make_context_evaluator(eval_string, context): (context) => any {
	let template = create_context_function_template(eval_string, context)
	let functor = Function(template)
	return functor()
}

export function evalValue(
	k: string,
	mod: FlowModule,
	testStepStore: Record<string, any>,
	pickableProperties: PickableProperties | undefined,
	showError: boolean
) {
	let inputTransforms = (mod.value['input_transforms'] ?? {}) as Record<string, InputTransform>
	let v = testStepStore[mod.id]?.[k]
	let t = inputTransforms?.[k]
	if (!v) {
		if (t.type == 'static') {
			v = t.value
		} else {
			try {
				let context = {
					flow_input: pickableProperties?.flow_input,
					results: pickableProperties?.priorIds
				}
				v = make_context_evaluator(t.expr, context)(context)
			} catch (e) {
				if (showError) {
					sendUserToast(`Error evaluating ${k}: ${e.message}`, true)
				}
				v = undefined
			}
		}
	}
	if (v == NEVER_TESTED_THIS_FAR) {
		return undefined
	}
	return v
}

export function filteredContentForExport(flow: ExtendedOpenFlow) {
	let o = {
		summary: flow.summary,
		description: flow.description,
		value: flow.value,
		schema: flow.schema
	}
	if (flow.dedicated_worker) {
		o['dedicated_worker'] = flow.dedicated_worker
	}
	if (flow.visible_to_runner_only) {
		o['visible_to_runner_only'] = flow.visible_to_runner_only
	}
	if (flow.on_behalf_of_email) {
		o['on_behalf_of_email'] = flow.on_behalf_of_email
	}
	if (flow.ws_error_handler_muted) {
		o['ws_error_handler_muted'] = flow.ws_error_handler_muted
	}
	if (flow.tag) {
		o['tag'] = flow.tag
	}
	return o
}

export function cleanInputs(flow: OpenFlow | any): OpenFlow & {
	tag?: string
	ws_error_handler_muted?: boolean
	dedicated_worker?: boolean
	visible_to_runner_only?: boolean
	on_behalf_of_email?: string
} {
	const newFlow: Flow = JSON.parse(JSON.stringify(flow))
	newFlow.value.modules.forEach((mod) => {
		if (mod.value.type == 'rawscript' || mod.value.type == 'script') {
			Object.values(mod.value.input_transforms ?? {}).forEach((inp) => {
				// for now we use the value for dynamic expression when done in the static editor so we have to resort to this
				if (inp.type == 'javascript') {
					//@ts-ignore
					inp.value = undefined
					inp.expr = cleanExpr(inp.expr)
				} else {
					//@ts-ignore
					inp.expr = undefined
				}
			})
		}
	})
	if (newFlow.value.concurrency_key == '') {
		newFlow.value.concurrency_key = undefined
	}

	return newFlow
}

export function getDefaultExpr(
	key: string = 'myfield',
	previousModuleId: string | undefined,
	previousExpr?: string
) {
	return (
		previousExpr ?? (previousModuleId ? `results.${previousModuleId}.${key}` : `flow_input.${key}`)
	)
}

export function jobsToResults(jobs: Job[]) {
	return jobs.map((job) => {
		if ('result' in job) {
			return job.result
		} else if (Array.isArray(job)) {
			return jobsToResults(job)
		}
	})
}

export async function runFlowPreview(
	args: Record<string, any>,
	flow: OpenFlow & { tag?: string },
	path: string,
	restartedFrom: RestartedFrom | undefined
) {
	const newFlow = flow
	return await JobService.runFlowPreview({
		workspace: get(workspaceStore) ?? '',
		requestBody: {
			args,
			value: newFlow.value,
			path: path,
			tag: newFlow.tag,
			restarted_from: restartedFrom
		}
	})
}

export function codeToStaticTemplate(code?: string): string | undefined {
	if (!code || typeof code != 'string') return undefined

	const lines = code.split('\n')
	if (lines.length == 1) {
		const line = lines[0].trim()
		if (line[0] == '`' && line.charAt(line.length - 1) == '`') {
			return line.slice(1, line.length - 1).replaceAll('\\`', '`')
		} else {
			return `\$\{${line}\}`
		}
	}
	return undefined
}

export function emptyFlowModuleState(): FlowModuleState {
	return {
		schema: emptySchema(),
		previewResult: NEVER_TESTED_THIS_FAR
	}
}

export function isInputFilled(
	inputTransforms: Record<string, InputTransform>,
	key: string,
	schema: Schema | undefined
): boolean {
	const required = schema?.required?.includes(key) ?? false

	if (!required) {
		return true
	}

	if (inputTransforms.hasOwnProperty(key)) {
		const transform = inputTransforms[key]
		if (
			transform?.type === 'static' &&
			(transform?.value === undefined || transform?.value === '' || transform?.value === null)
		) {
			return false
		} else if (
			transform?.type === 'javascript' &&
			(transform?.expr === undefined || transform?.expr === '' || transform?.expr === null)
		) {
			return false
		}
	}

	return true
}

async function isConnectedToMissingModule(
	argName: string,
	flowModuleValue: FlowModuleValue,
	moduleIds: string[]
): Promise<string | undefined> {
	const type = flowModuleValue.type

	if (type === 'rawscript' || type === 'script' || type === 'flow') {
		const input = flowModuleValue?.input_transforms[argName]
		const val: string = input.type === 'static' ? String(input.value) : input.expr

		try {
			const outputs = await parseOutputs(val, true)
			let error: string = ''

			outputs?.forEach(([componentId, id]) => {
				if (componentId === 'results') {
					if (!moduleIds.includes(id)) {
						error += `Input ${argName} is connected to a missing module with id ${id}\n`
					}
				}
			})

			return error
		} catch (e) {
			return `Input ${argName} expression is invalid`
		}
	}

	return
}

export async function computeFlowStepWarning(
	argName: string,
	flowModuleValue: FlowModuleValue,
	messages: Record<
		string,
		{
			message: string
			type: 'error' | 'warning'
		}
	>,
	schema: Schema | undefined,
	moduleIds: string[] = []
) {
	if (messages[argName]) {
		delete messages[argName]
	}

	const type = flowModuleValue.type
	if (type == 'rawscript' || type == 'script' || type == 'flow') {
		if (!isInputFilled(flowModuleValue.input_transforms, argName, schema)) {
			messages[argName] = {
				message: `Input ${argName} is required but not filled`,
				type: 'warning'
			}
		}

		const errorMessage = await isConnectedToMissingModule(argName, flowModuleValue, moduleIds)

		if (errorMessage) {
			messages[argName] = {
				message: errorMessage,
				type: 'error'
			}
		} else {
			if (messages[argName]?.type === 'error') {
				delete messages[argName]
			}
		}
	}

	return messages
}

export async function initFlowStepWarnings(
	flowModuleValue: FlowModuleValue,
	schema: Schema | undefined,
	moduleIds: string[] = []
) {
	const messages: Record<
		string,
		{
			message: string
			type: 'error' | 'warning'
		}
	> = {}
	const type = flowModuleValue.type

	if (type == 'rawscript' || type == 'script' || type == 'flow') {
		const keys = Object.keys(flowModuleValue.input_transforms ?? {})
		const promises = keys.map(async (key) => {
			await computeFlowStepWarning(key, flowModuleValue, messages, schema, moduleIds)
		})
		await Promise.all(promises)
	}

	return messages
}

export function checkIfParentLoop(
	flowStore: ExtendedOpenFlow,
	modId: string
): { id: string; type: 'forloopflow' | 'whileloopflow' } | undefined {
	const flow: ExtendedOpenFlow = JSON.parse(JSON.stringify(flowStore))
	const parents = dfs(modId, flow, true)
	for (const parent of parents.slice(1)) {
		if (parent.value.type === 'forloopflow' || parent.value.type === 'whileloopflow') {
			return { id: parent.id, type: parent.value.type }
		}
	}
	return undefined
}

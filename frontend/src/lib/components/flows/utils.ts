import {
	JobService,
	ScriptService,
	type Flow,
	type FlowModule,
	type InputTransform,
	type Job,
	type RestartedFrom,
	type OpenFlow
} from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { cleanExpr, emptySchema } from '$lib/utils'
import { get } from 'svelte/store'
import type { FlowModuleState } from './flowState'
import type { PickableProperties } from './previousResults'
import { NEVER_TESTED_THIS_FAR } from './models'
import { sendUserToast } from '$lib/toast'

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

export function cleanInputs(
	flow: OpenFlow | any
): OpenFlow & { tag?: string; ws_error_handler_muted?: boolean; dedicated_worker?: boolean } {
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

import type { Schema } from '$lib/common'
import type { InputTransform, OpenFlow } from '$lib/gen'
import { parseOutputs } from '$lib/infer'
import type { Writable } from 'svelte/store'
import type { FlowInput } from './types'
import type { StateStore } from '$lib/utils'
import type { FlowState } from './flowState'
import { dfs } from './dfs'

function isInputFilled(
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
	input_transform: InputTransform,
	moduleIds: string[]
): Promise<string | undefined> {
	const val: string =
		input_transform.type === 'static' ? String(input_transform.value) : input_transform.expr

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

type FlowStepWarnings = Record<string, { message: string; type: 'error' | 'warning' }>

async function computeFlowStepWarnings(
	input: {
		input_transforms: Record<string, InputTransform>
		id: string
		schema: Schema | undefined
	},
	moduleIds: string[] = []
) {
	const messages: FlowStepWarnings = {}
	const { input_transforms, schema } = input
	const keys = Object.keys(input_transforms ?? {})
	const promises = keys.map(async (key) => {
		if (!isInputFilled(input_transforms, key, schema)) {
			messages[key] = {
				message: `Input ${key} is required but not filled`,
				type: 'warning'
			}
		} else {
			const errorMessage = await isConnectedToMissingModule(key, input_transforms[key], moduleIds)
			if (errorMessage) {
				messages[key] = {
					message: errorMessage,
					type: 'error'
				}
			}
		}
	})
	await Promise.all(promises)

	return messages
}

export async function computeMissingInputWarnings(
	flowStore: StateStore<OpenFlow>,
	flowState: FlowState,
	flowInputsStore: Writable<FlowInput>
) {
	const inputs = dfs(flowStore.val.value.modules, (module) => {
		if (
			module.value.type === 'script' ||
			module.value.type === 'rawscript' ||
			module.value.type === 'flow'
		) {
			const schema = flowState[module.id]?.schema
			return {
				input_transforms: module.value.input_transforms,
				id: module.id,
				schema
			}
		}
		return undefined
	}).filter((x) => x !== undefined)

	const moduleIds = dfs(flowStore.val.value.modules, (module) => module.id)

	const promises = inputs.map(async (input) => {
		const warnings = await computeFlowStepWarnings(input, moduleIds)
		return [input.id, warnings] as const
	})
	const warnings = Object.fromEntries(await Promise.all(promises))
	for (const key in warnings) {
		flowInputsStore.update((fi) => {
			return {
				...fi,
				[key]: {
					flowStepWarnings: warnings[key]
				}
			}
		})
	}
}

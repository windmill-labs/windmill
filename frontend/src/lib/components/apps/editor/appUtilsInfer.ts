import { parseOutputs } from '$lib/infer'
import { deepEqual } from 'fast-equals'
import type { EvalV2AppInput } from '../inputType'
import type { Writable } from 'svelte/store'
import type { App } from '../types'

export async function inferDeps(
	code: string,
	worldOutputs: Record<string, any>,
	componentInput: EvalV2AppInput,
	app: Writable<App>
) {
	const outputs = await parseOutputs(code, true)
	if (outputs && componentInput) {
		const noutputs = outputs
			.filter(([key, id]) => key == 'row' || key == 'iter' || id in (worldOutputs[key] ?? {}))
			.map(([key, id]) => ({
				componentId: key,
				id: id
			}))
		if (!deepEqual(noutputs, componentInput.connections)) {
			componentInput.connections = noutputs
			app.update((old) => old)
		}
	}
}

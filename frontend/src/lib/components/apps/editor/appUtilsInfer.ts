import { parseOutputs } from '$lib/infer'
import { deepEqual } from 'fast-equals'
import type { EvalV2AppInput, TemplateV2Input } from '../inputType'
import type { Writable } from 'svelte/store'
import type { App } from '../types'

export async function inferDeps(
	code: string,
	worldOutputs: Record<string, any>,
	componentInput: EvalV2AppInput | TemplateV2Input,
	app: Writable<App>
) {
	const outputs = await parseOutputs(code, true)
	if (outputs && componentInput) {
		const noutputs = outputs
			.filter(
				([componentId, id]) =>
					componentId == 'row' ||
					componentId == 'iter' ||
					componentId == 'group' ||
					componentId == 'file' ||
					id in (worldOutputs[componentId] ?? {})
			)
			.map(([componentId, id]) => ({
				componentId: componentId,
				id: id
			}))
		if (!deepEqual(noutputs, componentInput.connections)) {
			componentInput.connections = noutputs
			app.update((old) => old)
		}
	}
}

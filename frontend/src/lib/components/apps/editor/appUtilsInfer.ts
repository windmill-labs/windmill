import { parseOutputs } from '$lib/infer'
import { deepEqual } from 'fast-equals'
import type { EvalV2AppInput, TemplateV2Input } from '../inputType'
import type { App } from '../types'
import type { StateStore } from '$lib/utils'

export async function inferDeps(
	code: string,
	worldOutputs: Record<string, any>,
	componentInput: EvalV2AppInput | TemplateV2Input,
	app: StateStore<App>
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
					componentId == 'state' ||
					id in (worldOutputs[componentId] ?? {})
			)
			.map(([componentId, id]) => ({
				componentId: componentId,
				id: id
			}))
		if (!deepEqual(noutputs, componentInput.connections)) {
			componentInput.connections = noutputs
			app.val = app.val
		}
	}
}

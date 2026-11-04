import type { PickableProperties } from '../flows/previousResults'

/** The flow input an AI-suggested step input expression names when the flow has no such input
 * yet, or undefined when it names none. */
export function newFlowInputOf(
	expr: string,
	pickableProperties: PickableProperties | undefined
): string | undefined {
	if (!pickableProperties || !expr.startsWith('flow_input.')) return undefined
	const key = expr.split('.')[1]
	return key && !(key in pickableProperties.flow_input) ? key : undefined
}

export function newFlowInputsInstruction(canAddFlowInputs: boolean, omitHint: string): string {
	return canAddFlowInputs
		? 'If none of the available results are appropriate, are already used or are more appropriate for other inputs, you can also imagine new flow_input properties which we will create programmatically based on what you provide.'
		: `Only use the flow_input properties listed in the available data: new flow inputs cannot be created. If none of the available data is appropriate, ${omitHint}.`
}

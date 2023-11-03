import type { OpenFlow } from '$lib/gen'
import { dfs } from './dfs'
import type { FlowState } from './flowState'
import { charsToNumber, numberToChars } from './idUtils'

// Computes the next available id
export function nextId(flowState: FlowState, fullFlow: OpenFlow): string {
	const allIds = dfs(fullFlow.value.modules, (fm) => fm.id)
	const max = allIds.concat(Object.keys(flowState)).reduce((acc, key) => {
		if (key === 'failure' || key.includes('branch') || key.includes('loop')) {
			return acc
		} else {
			const num = charsToNumber(key)
			return Math.max(acc, num + 1)
		}
	}, 0)
	return numberToChars(max)
}

import type { OpenFlow } from '$lib/gen'
import { dfs } from './dfs'
import type { FlowState } from './flowState'
import { charsToNumber, numberToChars } from './idUtils'

// Computes the next available id
export function nextId(flowState: FlowState, fullFlow: OpenFlow): string {
	const allIds = dfs(fullFlow.value.modules, (fm) => fm.id)

	const max = allIds.concat(Object.keys(flowState)).reduce((acc, key) => {
		if (key.length >= 4) {
			return acc
		} else {
			const num = charsToNumber(key)
			return Math.max(acc, num + 1)
		}
	}, 0)
	return numberToChars(max)
}

// Computes a copy id like "a2", "a3", etc. based on the original id
export function copyId(originalId: string, flowState: FlowState, fullFlow: OpenFlow): string {
	const allIds = new Set(dfs(fullFlow.value.modules, (fm) => fm.id).concat(Object.keys(flowState)))
	for (let n = 2; ; n++) {
		const candidate = `${originalId}${n}`
		if (!allIds.has(candidate)) {
			return candidate
		}
	}
}

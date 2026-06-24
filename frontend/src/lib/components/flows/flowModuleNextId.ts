import type { OpenFlow } from '$lib/gen'
import { dfs } from './dfs'
import type { FlowState } from './flowState'
import { charsToNumber, forbiddenIds, numberToChars } from './idUtils'

const reservedIds = new Set(forbiddenIds)

// Only auto-generated ids (canonical base-26 lowercase sequences like a, b, ..., z, aa, ...)
// may contribute to the next-id computation. flowState/module-id keys also include copy ids
// ("a2"), subflow result keys ("subflow:..."), reserved keys ("failure", "preprocessor") and
// user-renamed ids; feeding those through charsToNumber yields meaningless (often huge) numbers
// that would poison id generation, making new steps jump to ids like "bzw".
function autoIdNumber(key: string): number | undefined {
	if (reservedIds.has(key)) {
		return undefined
	}
	const num = charsToNumber(key)
	if (num < 0 || numberToChars(num) !== key) {
		return undefined
	}
	return num
}

// Computes the next available id
export function nextId(flowState: FlowState, fullFlow: OpenFlow): string {
	const allIds = dfs(fullFlow.value.modules, (fm) => fm.id)

	const max = allIds.concat(Object.keys(flowState)).reduce((acc, key) => {
		const num = autoIdNumber(key)
		return num === undefined ? acc : Math.max(acc, num + 1)
	}, 0)
	return numberToChars(max)
}

// Computes a copy id like "a2", "a3", etc. based on the original id
export function copyId(originalId: string, flowState: FlowState, fullFlow: OpenFlow): string {
	const allIds = new Set(dfs(fullFlow.value.modules, (fm) => fm.id).concat(Object.keys(flowState)))
	for (let n = 2; n < 10000; n++) {
		const candidate = `${originalId}${n}`
		if (!allIds.has(candidate)) {
			return candidate
		}
	}
	return `${originalId}10000`
}

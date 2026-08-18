import type { OpenFlow } from '$lib/gen'
import { dfs } from './dfs'
import type { FlowState } from './flowState'
import { charsToNumber, forbiddenIds, numberToChars } from './idUtils'

const reservedIds = new Set(forbiddenIds)

// Returns the base-26 value of a key only if it is a short, auto-generated step id
// (a, b, ..., z, aa, ...). flowState/module-id keys also include copy ids ("a2"), subflow
// result keys ("subflow:..."), reserved keys and user-renamed ids; feeding those through
// charsToNumber yields meaningless (often huge) numbers that would poison id generation and
// make new steps jump to ids like "bzw". Short non-canonical keys are rejected via a
// round-trip check; longer keys are skipped entirely, which also leaves user renames to long
// lowercase words (e.g. "process") out of the sequence.
function autoIdNumber(key: string): number | undefined {
	if (key.length >= 4 || reservedIds.has(key)) {
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
	const allKeys = dfs(fullFlow.value.modules, (fm) => fm.id).concat(Object.keys(flowState))
	const takenIds = new Set(allKeys)

	const max = allKeys.reduce((acc, key) => {
		const num = autoIdNumber(key)
		return num === undefined ? acc : Math.max(acc, num + 1)
	}, 0)

	// A reserved id (e.g. "as", after "ar") never raises `max`, so assigning
	// numberToChars(max) when it is reserved would make every later call return
	// it again; skip forward to a free, allowed id instead.
	let candidate = max
	let id = numberToChars(candidate)
	while (reservedIds.has(id) || takenIds.has(id)) {
		id = numberToChars(++candidate)
	}
	return id
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

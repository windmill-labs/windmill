// Created this utility to avoid circular dependencies between
// svelte effects

import { deepEqual } from 'fast-equals'

export function createOnObjChange<T extends object>() {
	let oldValue: T | undefined

	return (value: T | undefined, callback: () => void) => {
		if (!deepEqual(oldValue, value)) {
			oldValue = structuredClone(value)
			callback()
		}
	}
}

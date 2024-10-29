import { deepEqual } from 'fast-equals'

export function isWindmillTooBigObject(obj: any): boolean {
	return (
		typeof obj === 'object' &&
		deepEqual(Object.keys(obj), ['reason']) &&
		obj['reason'] == 'WINDMILL_TOO_BIG'
	)
}

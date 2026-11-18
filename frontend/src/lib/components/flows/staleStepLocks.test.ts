import { describe, it, expect } from 'vitest'
import { dropStaleStepLocks } from './staleStepLocks'

const step = (id: string, content: string, lock?: string) =>
	({ id, value: { type: 'rawscript', language: 'python3', content, lock } }) as any

describe('dropStaleStepLocks', () => {
	// A rename or duplicate gives the step a new id; its code and lock still match.
	it('keeps the lock of a renamed or duplicated step whose code is unchanged', () => {
		const base = { modules: [step('a', 'import requests', 'requests==2.0')] }
		const value = {
			modules: [
				step('a', 'import requests', 'requests==2.0'),
				step('a_copy', 'import requests', 'requests==2.0'),
				step('a_edited_copy', 'import yaml', 'requests==2.0')
			]
		}
		dropStaleStepLocks(value as any, base as any)
		expect(value.modules.map((m) => m.value.lock)).toEqual([
			'requests==2.0',
			'requests==2.0',
			undefined
		])
	})
})

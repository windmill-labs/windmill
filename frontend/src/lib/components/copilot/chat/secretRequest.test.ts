import { describe, expect, it } from 'vitest'
import { secretRequestToolResult } from './secretRequest'

// The model reads this back: it must name where the secret is, and nothing else of it.
describe('secretRequestToolResult', () => {
	it('gives the path the user saved to, and says so when it is not the one asked for', () => {
		const same = JSON.parse(
			secretRequestToolResult({ path: 'u/a/key', replaced: false }, 'u/a/key')
		)
		expect(same).toMatchObject({ success: true, variable: '$var:u/a/key' })
		expect(same.message).not.toContain('instead of')

		const moved = JSON.parse(
			secretRequestToolResult({ path: 'u/a/key_2', replaced: false }, 'u/a/key')
		)
		expect(moved.variable).toBe('$var:u/a/key_2')
		expect(moved.message).toContain('instead of u/a/key')
	})

	it('reports a dismissal as a failure with no variable', () => {
		const dismissed = JSON.parse(secretRequestToolResult(undefined, 'u/a/key'))
		expect(dismissed.success).toBe(false)
		expect(dismissed.variable).toBeUndefined()
	})
})

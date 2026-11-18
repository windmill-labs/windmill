import { describe, it, expect } from 'vitest'
import { extractMarkedLabel } from './instanceSettings'

// SearchItems escapes the haystack before highlighting, so `marked` carries
// entities where the label had `& < > " '`. The offset walk counts unescaped
// characters, so an entity must advance it by one — otherwise the label is
// truncated early and can be sliced mid-entity.
describe('extractMarkedLabel', () => {
	it('counts an escaped character as one, not as its entity length', () => {
		const label = 'A & B'
		expect(extractMarkedLabel('A &amp; B — category', label.length)).toBe('A &amp; B')
	})

	it('keeps the mark wrapper and stops at the label boundary', () => {
		const label = 'Base URL'
		expect(extractMarkedLabel('<mark>Base</mark> URL — general', label.length)).toBe(
			'<mark>Base</mark> URL'
		)
	})

	it('never slices an entity in half', () => {
		const label = '"q" & <x>'
		const out = extractMarkedLabel('&quot;q&quot; &amp; &lt;x&gt; — trailing', label.length)
		expect(out).toBe('&quot;q&quot; &amp; &lt;x&gt;')
		expect(out.endsWith(';')).toBe(true)
	})
})

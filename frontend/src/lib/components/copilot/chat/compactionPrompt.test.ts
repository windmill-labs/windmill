import { describe, expect, it } from 'vitest'
import {
	buildSummaryMessageContent,
	formatCompactSummary,
	getCompactionSummaryPrompt
} from './compactionPrompt'

describe('formatCompactSummary', () => {
	it('strips the analysis scratchpad and unwraps the summary block', () => {
		const raw = `<analysis>
chronological thinking the model should not keep
</analysis>
<summary>
1. Primary Request and Intent: build the thing
2. Pending Tasks: none
</summary>`
		const formatted = formatCompactSummary(raw)
		expect(formatted).not.toContain('chronological thinking')
		expect(formatted).not.toContain('<analysis>')
		expect(formatted).not.toContain('<summary>')
		expect(formatted).toContain('Primary Request and Intent: build the thing')
	})

	it('falls back to the trimmed raw text when the model omits the tags', () => {
		expect(formatCompactSummary('  just a plain summary  ')).toBe('just a plain summary')
	})

	it('keeps the summary even when there is no analysis block', () => {
		expect(formatCompactSummary('<summary>only the summary</summary>')).toBe('only the summary')
	})

	it('collapses the blank-line runs left by stripping analysis', () => {
		const raw = '<analysis>x</analysis>\n\n\n\n<summary>a\n\n\n\nb</summary>'
		expect(formatCompactSummary(raw)).toBe('a\n\nb')
	})

	it('strips every analysis block, not just the first, when the summary is untagged', () => {
		const raw = '<analysis>first</analysis>\nkept one\n<analysis>second</analysis>\nkept two'
		const formatted = formatCompactSummary(raw)
		expect(formatted).not.toContain('first')
		expect(formatted).not.toContain('second')
		expect(formatted).not.toContain('<analysis>')
		expect(formatted).toContain('kept one')
		expect(formatted).toContain('kept two')
	})
})

describe('buildSummaryMessageContent', () => {
	it('embeds the summary and frames it as a continuation', () => {
		const content = buildSummaryMessageContent('THE SUMMARY')
		expect(content).toContain('THE SUMMARY')
		expect(content).toContain('continued from a previous conversation')
		expect(content).toContain('preserved verbatim')
	})
})

describe('getCompactionSummaryPrompt', () => {
	it('asks for a structured, text-only summary', () => {
		const prompt = getCompactionSummaryPrompt()
		expect(prompt).toContain('detailed summary')
		expect(prompt).toContain('<summary>')
		expect(prompt).toContain('TEXT ONLY')
	})
})

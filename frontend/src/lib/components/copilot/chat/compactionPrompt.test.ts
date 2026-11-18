import { describe, expect, it } from 'vitest'
import {
	buildSummaryMessageContent,
	formatCompactSummary,
	getCompactionSummaryPrompt,
	toolExchangesAsText
} from './compactionPrompt'

describe('toolExchangesAsText', () => {
	it('renders tool calls and results as plain text messages', () => {
		const sent = toolExchangesAsText([
			{ role: 'user', content: 'weather?' },
			{
				role: 'assistant',
				content: null,
				tool_calls: [
					{
						id: 'call_1',
						type: 'function',
						function: { name: 'get_weather', arguments: '{"city":"Paris"}' }
					}
				]
			},
			{ role: 'tool', tool_call_id: 'call_1', content: 'sunny' },
			{ role: 'assistant', content: 'It is sunny.' }
		])
		expect(sent.every((m) => m.role !== 'tool' && !('tool_calls' in m))).toBe(true)
		expect(sent[1]).toEqual({
			role: 'assistant',
			content: '[Called tool `get_weather` with arguments: {"city":"Paris"}]'
		})
		expect(sent[2]).toEqual({ role: 'user', content: '[Result of tool `get_weather`: sunny]' })
		expect(sent[3]).toEqual({ role: 'assistant', content: 'It is sunny.' })
	})
})

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

	it('keeps the summary content when <summary> has no closing tag', () => {
		const raw = '<summary>\n1. Primary Request and Intent: build the thing\n2. Pending Tasks: none'
		const formatted = formatCompactSummary(raw)
		expect(formatted).not.toContain('<summary>')
		expect(formatted).toContain('Primary Request and Intent: build the thing')
		expect(formatted).toContain('Pending Tasks: none')
	})

	it('drops the analysis scratchpad even when <summary> is left unclosed', () => {
		const raw =
			'<analysis>\nchronological thinking the model should not keep\n</analysis>\n<summary>\nthe real summary'
		const formatted = formatCompactSummary(raw)
		expect(formatted).not.toContain('chronological thinking')
		expect(formatted).not.toContain('<analysis>')
		expect(formatted).not.toContain('<summary>')
		expect(formatted).toBe('the real summary')
	})

	it('strips an orphaned closing summary tag', () => {
		expect(formatCompactSummary('plain summary</summary>')).toBe('plain summary')
	})

	it('does not leak analysis scratchpad that mentions a literal <summary> tag', () => {
		const raw = `<analysis>scratchpad mentions <summary> before output</analysis>
<summary>real summary</summary>`
		const formatted = formatCompactSummary(raw)
		expect(formatted).toBe('real summary')
		expect(formatted).not.toContain('scratchpad')
		expect(formatted).not.toContain('before output')
	})

	it('keeps the whole summary when it quotes its own tags', () => {
		const raw = `<analysis>notes</analysis>
<summary>1. The user asked for an <analysis> block then a <summary></summary> block.
2. Work continued.</summary>`
		expect(formatCompactSummary(raw)).toBe(
			'1. The user asked for an  block then a  block.\n2. Work continued.'
		)
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

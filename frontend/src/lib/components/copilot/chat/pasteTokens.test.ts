import { describe, expect, it } from 'vitest'
import {
	type PasteAttachment,
	countLines,
	expandPasteTokens,
	lineCountLabel,
	makePasteToken,
	nextPasteId,
	shouldCollapsePaste,
	splitPasteTokens
} from './pasteTokens'

const att = (id: number, lines: number, content: string): PasteAttachment => ({
	id,
	lines,
	content
})

describe('lineCountLabel', () => {
	it('pluralizes by count', () => {
		expect(lineCountLabel(1)).toBe('1 line')
		expect(lineCountLabel(0)).toBe('0 lines')
		expect(lineCountLabel(13)).toBe('13 lines')
	})
})

describe('countLines', () => {
	it('counts display lines', () => {
		expect(countLines('a')).toBe(1)
		expect(countLines('a\nb\nc')).toBe(3)
	})
	it('ignores a single trailing newline', () => {
		expect(countLines('a\nb\n')).toBe(2)
		expect(countLines('a\n')).toBe(1)
	})
})

describe('shouldCollapsePaste', () => {
	it('collapses past the line threshold', () => {
		expect(shouldCollapsePaste('a\n'.repeat(11))).toBe(true)
		expect(shouldCollapsePaste('a\n'.repeat(9))).toBe(false)
	})
	it('does not collapse a 10-line paste with a trailing newline', () => {
		// 10 lines + trailing newline must not read as 11 (the off-by-one).
		expect(shouldCollapsePaste(Array(10).fill('x').join('\n') + '\n')).toBe(false)
		expect(shouldCollapsePaste(Array(11).fill('x').join('\n') + '\n')).toBe(true)
	})
	it('collapses very long single-line blobs', () => {
		expect(shouldCollapsePaste('x'.repeat(1001))).toBe(true)
		expect(shouldCollapsePaste('x'.repeat(500))).toBe(false)
	})
})

describe('token round-trip', () => {
	it('expands a token back to its full content', () => {
		const a = att(1, 12, 'line1\nline2')
		const text = `before ${makePasteToken(a)} after`
		expect(expandPasteTokens(text, [a])).toBe('before line1\nline2 after')
	})

	it('maps duplicate-label tokens to the right blob via the zero-width id', () => {
		const a = att(1, 12, 'AAA')
		const b = att(2, 12, 'BBB') // same line count → identical visible label
		const text = `${makePasteToken(a)} and ${makePasteToken(b)}`
		expect(expandPasteTokens(text, [a, b])).toBe('AAA and BBB')
	})

	it('leaves unknown tokens untouched', () => {
		const a = att(1, 12, 'AAA')
		const token = makePasteToken(a)
		expect(expandPasteTokens(token, [])).toBe(token)
	})
})

describe('splitPasteTokens', () => {
	it('splits into text and paste segments in order', () => {
		const a = att(1, 12, 'AAA')
		const segs = splitPasteTokens(`hi ${makePasteToken(a)} bye`, [a])
		expect(segs).toEqual([
			{ type: 'text', value: 'hi ' },
			{ type: 'paste', att: a },
			{ type: 'text', value: ' bye' }
		])
	})

	it('returns a single text segment when there are no pastes', () => {
		expect(splitPasteTokens('plain', [])).toEqual([{ type: 'text', value: 'plain' }])
	})
})

describe('nextPasteId', () => {
	it('is unique and monotonic', () => {
		expect(nextPasteId([])).toBe(1)
		expect(nextPasteId([att(1, 1, 'a'), att(3, 1, 'b')])).toBe(4)
	})
})

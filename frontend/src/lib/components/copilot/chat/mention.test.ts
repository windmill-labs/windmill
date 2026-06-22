import { describe, expect, it } from 'vitest'
import { MENTION_RE, mentionTitle, formatMention } from './mention'

describe('formatMention', () => {
	it('leaves a simple name bare', () => {
		expect(formatMention('app.ts')).toBe('@app.ts')
		expect(formatMention('proj/sub/a.ts')).toBe('@proj/sub/a.ts')
	})
	it('brackets a name containing whitespace', () => {
		expect(formatMention('my file.txt')).toBe('@[my file.txt]')
		expect(formatMention('my folder/a b.ts')).toBe('@[my folder/a b.ts]')
	})
	it('brackets names with HTML-sensitive chars, parens, brackets', () => {
		expect(formatMention('R&D notes.md')).toBe('@[R&D notes.md]')
		expect(formatMention('a<b>.txt')).toBe('@[a<b>.txt]')
		expect(formatMention('report(final).csv')).toBe('@[report(final).csv]')
	})
})

describe('mentionTitle', () => {
	it('strips the @ from a bare mention', () => {
		expect(mentionTitle('@app.ts')).toBe('app.ts')
	})
	it('strips the @[ ] from a bracketed mention', () => {
		expect(mentionTitle('@[my file.txt]')).toBe('my file.txt')
	})
})

describe('MENTION_RE', () => {
	it('captures a bracketed (spaced) mention whole alongside bare ones', () => {
		const tokens = [...'see @app.ts and @[my file.txt] ok'.matchAll(MENTION_RE)].map((m) => m[0])
		expect(tokens).toEqual(['@app.ts', '@[my file.txt]'])
		expect(tokens.map(mentionTitle)).toEqual(['app.ts', 'my file.txt'])
	})
	it('round-trips formatMention → MENTION_RE → mentionTitle for a spaced name', () => {
		const name = 'my notes (v2).md'
		const m = `x ${formatMention(name)} y`.match(MENTION_RE)!
		expect(mentionTitle(m[0])).toBe(name)
	})

	it('round-trips a name containing both whitespace and a closing bracket', () => {
		const name = 'notes ] draft.md'
		expect(formatMention(name)).toBe('@[notes \\] draft.md]')
		const m = `x ${formatMention(name)} y`.match(MENTION_RE)!
		expect(m[0]).toBe('@[notes \\] draft.md]')
		expect(mentionTitle(m[0])).toBe(name)
	})

	it('round-trips an HTML-sensitive name (highlighter handles HTML-escaping separately)', () => {
		const name = 'a <b> & c].txt'
		const m = `x ${formatMention(name)} y`.match(MENTION_RE)!
		expect(mentionTitle(m[0])).toBe(name)
	})
})

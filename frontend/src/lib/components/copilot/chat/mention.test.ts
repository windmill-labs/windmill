import { describe, expect, it } from 'vitest'
import {
	MENTION_RE,
	mentionTitle,
	formatMention,
	hasMention,
	mentionTitlesInText,
	removeMentionFromText
} from './mention'

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

describe('removeMentionFromText', () => {
	it('removes a bare mention and keeps neighboring words separated', () => {
		expect(removeMentionFromText('before @app.ts after', 'app.ts')).toBe('before after')
	})

	it('removes a bracketed mention', () => {
		expect(removeMentionFromText('before @[my folder/a b.ts] after', 'my folder/a b.ts')).toBe(
			'before after'
		)
	})

	it('removes an escaped bracketed mention', () => {
		expect(removeMentionFromText('before @[notes \\] draft.md] after', 'notes ] draft.md')).toBe(
			'before after'
		)
	})

	it('removes edge mentions without leaving extra whitespace', () => {
		expect(removeMentionFromText('@app.ts after', 'app.ts')).toBe('after')
		expect(removeMentionFromText('before @app.ts', 'app.ts')).toBe('before')
	})

	it('keeps embedded mention-like text intact', () => {
		expect(removeMentionFromText('Contact owner@app.ts about @app.ts', 'app.ts')).toBe(
			'Contact owner@app.ts about'
		)
	})

	it('keeps Unicode-prefixed embedded mention-like text intact', () => {
		expect(removeMentionFromText('Contact 用户@app.ts about @app.ts', 'app.ts')).toBe(
			'Contact 用户@app.ts about'
		)
		expect(removeMentionFromText('Contact हिंदी@app.ts about @app.ts', 'app.ts')).toBe(
			'Contact हिंदी@app.ts about'
		)
		expect(removeMentionFromText('Contact 𐐀@app.ts about @app.ts', 'app.ts')).toBe(
			'Contact 𐐀@app.ts about'
		)
	})
})

describe('hasMention', () => {
	it('does not treat embedded mention-like text as a selected context mention', () => {
		expect(hasMention('Contact owner@app.ts', 'app.ts')).toBe(false)
		expect(hasMention('Contact owner@app.ts about @app.ts', 'app.ts')).toBe(true)
	})

	it('does not treat Unicode-prefixed embedded text as a mention', () => {
		expect(hasMention('Contact 用户@app.ts', 'app.ts')).toBe(false)
		expect(hasMention('Contact हिंदी@app.ts', 'app.ts')).toBe(false)
		expect(hasMention('Contact 𐐀@app.ts', 'app.ts')).toBe(false)
	})

	it('treats punctuation next to a mention as a boundary', () => {
		expect(hasMention('use @app.ts, then compare', 'app.ts')).toBe(true)
		expect(hasMention('open (@app.ts)', 'app.ts')).toBe(true)
	})
})

describe('mentionTitlesInText', () => {
	it('extracts only standalone mentions for context synchronization', () => {
		expect([...mentionTitlesInText('Contact owner@app.ts about @app.ts')]).toEqual(['app.ts'])
		expect([...mentionTitlesInText('Contact owner@app.ts')]).toEqual([])
	})

	it('ignores Unicode-prefixed embedded text for context synchronization', () => {
		expect([...mentionTitlesInText('Contact 用户@app.ts')]).toEqual([])
	})

	it('keeps punctuation-adjacent mentions synchronized', () => {
		expect([...mentionTitlesInText('use @app.ts, then compare')]).toEqual(['app.ts'])
		expect([...mentionTitlesInText('open (@app.ts)')]).toEqual(['app.ts'])
	})

	it('ignores combining-mark and astral-letter embedded text', () => {
		expect([...mentionTitlesInText('Contact हिंदी@app.ts')]).toEqual([])
		expect([...mentionTitlesInText('Contact 𐐀@app.ts')]).toEqual([])
	})
})

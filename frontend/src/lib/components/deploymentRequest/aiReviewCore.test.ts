import { describe, expect, it } from 'vitest'
import { parseReviewAnswer } from './aiReviewCore'

function answer(findings: object[]): string {
	return '```json\n' + JSON.stringify({ summary: 's', findings }) + '\n```'
}

describe('parseReviewAnswer', () => {
	it('anchors only findings on items in the diff, and ranks by severity', () => {
		const { findings } = parseReviewAnswer(
			answer([
				{ kind: 'flow', path: 'f/not/changed', severity: 'low', comment: 'phantom' },
				{ kind: 'script', path: 'f/a/b', severity: 'high', comment: 'real' }
			]),
			{ items: new Set(['script:f/a/b']) }
		)
		expect(findings.map((f) => [f.comment, f.kind, f.path])).toEqual([
			['real', 'script', 'f/a/b'],
			['phantom', '', '']
		])
	})

	// A pull request review is refused whole when one comment sits off the diff, so a line
	// the diff does not show must fall back to the file, and an unknown file to no anchor.
	it('keeps a line anchor only on a line the diff shows, on the side named', () => {
		const lines = new Map([['f/a/b.ts', { LEFT: new Set([3]), RIGHT: new Set([4, 5]) }]])
		const { findings } = parseReviewAnswer(
			answer([
				{ kind: 'script', path: 'f/a/b', file: 'f/a/b.ts', line: 'R5', comment: 'head' },
				{ kind: 'script', path: 'f/a/b', file: 'f/a/b.ts', line: 'L3', comment: 'base' },
				{ kind: 'script', path: 'f/a/b', file: 'f/a/b.ts', line: 'R3', comment: 'off diff' },
				{ kind: 'script', path: 'f/a/b', file: 'f/a/c.ts', line: 'R4', comment: 'other file' }
			]),
			{ items: new Set(['script:f/a/b']), lines }
		)
		expect(findings.map((f) => [f.comment, f.file, f.side, f.line])).toEqual([
			['head', 'f/a/b.ts', 'RIGHT', 5],
			['base', 'f/a/b.ts', 'LEFT', 3],
			['off diff', 'f/a/b.ts', undefined, undefined],
			['other file', undefined, undefined, undefined]
		])
	})
})

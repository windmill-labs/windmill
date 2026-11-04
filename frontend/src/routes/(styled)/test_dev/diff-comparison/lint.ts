import type { Marker } from '@pierre/diffs/edit'

type Rule = { pattern: RegExp; severity: Marker['severity']; message: string }

const RULES: Rule[] = [
	{ pattern: /(?<![=!])==(?!=)/g, severity: 'error', message: 'Expected === instead of ==.' },
	{ pattern: /\bvar\b/g, severity: 'warning', message: 'Unexpected var, use let or const.' },
	{ pattern: /\bconsole\.log\b|\bprint\(/g, severity: 'info', message: 'Debug output left in.' },
	{ pattern: /\bTODO\b/g, severity: 'hint', message: 'Unresolved TODO.' }
]

/** A stand-in for a language server: enough to show how markers render and re-anchor. */
export function lint(code: string): Marker[] {
	const markers: Marker[] = []
	code.split('\n').forEach((text, line) => {
		for (const rule of RULES) {
			for (const m of text.matchAll(rule.pattern)) {
				const start = m.index ?? 0
				markers.push({
					severity: rule.severity,
					message: rule.message,
					source: 'demo-lint',
					start: { line, character: start },
					end: { line, character: start + m[0].length }
				})
			}
		}
	})
	return markers
}

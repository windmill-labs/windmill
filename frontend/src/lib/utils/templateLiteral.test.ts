import { describe, it, expect } from 'vitest'
import { escapeTemplateBackticks, unescapeTemplateBackticks } from './templateLiteral'

// Template mode stores its value as a JS template literal, so backticks in the text have to be
// escaped. Escaping them inside `${...}` too is what broke nested template literals: a backslash
// is a syntax error in expression position.
describe('escapeTemplateBackticks', () => {
	const nested =
		'--input ${flow_input.iter.value}${results.config ? ` --config ${results.config}` : ""} --host ${flow_input.hostname}'

	it('leaves a nested template literal inside an interpolation intact', () => {
		const expr = '`' + escapeTemplateBackticks(nested) + '`'
		expect(expr).not.toContain('\\`')
		expect(
			new Function('flow_input', 'results', 'return ' + expr)(
				{ iter: { value: 'data.csv' }, hostname: 'host1' },
				{ config: '/tmp/cfg.json' }
			)
		).toBe('--input data.csv --config /tmp/cfg.json --host host1')
	})

	it('still escapes a backtick in the literal text', () => {
		expect(escapeTemplateBackticks('a ` b')).toBe('a \\` b')
		expect(escapeTemplateBackticks('a ` ${x} ` b')).toBe('a \\` ${x} \\` b')
	})

	it('leaves an escaped interpolation as literal text', () => {
		// `\\${...}` is escaped in the template source, so the backticks inside it are literal
		// text and still need escaping.
		expect(escapeTemplateBackticks('\\${foo `bar`}')).toBe('\\${foo \\`bar\\`}')
		expect(unescapeTemplateBackticks('\\${foo \\`bar\\`}')).toBe('\\${foo `bar`}')
		expect(
			() => new Function('return `' + escapeTemplateBackticks('\\${foo `bar`}') + '`')
		).not.toThrow()
	})

	// Braces, quotes, regex literals and comments all hide backticks and braces from anything
	// short of a real lexer, which is why the parser decides.
	it('handles text only a lexer can read correctly', () => {
		const inputs = [
			'${ x["}"] } `',
			"${ f({ a: '`' }) } `",
			"${ x.replace(/'/g, '') } `",
			'${ /* ` */ x } `',
			'{"match": ${/{/.test(flow_input.x)}, "literal": "`x`"}'
		]
		for (const v of inputs) {
			expect(() => new Function('return `' + escapeTemplateBackticks(v) + '`')).not.toThrow()
			expect(unescapeTemplateBackticks(escapeTemplateBackticks(v))).toBe(v)
		}
	})

	// The failure that matters most is not a syntax error but literal text quietly becoming code:
	// here the author's `+ flow_input.y +` must stay text rather than being evaluated.
	it('never lets literal text escape into the expression', () => {
		const v = '{"match": ${/{/.test(flow_input.x)}, "literal": "` + flow_input.y + `"}'
		const evaluated = new Function('flow_input', 'return `' + escapeTemplateBackticks(v) + '`')({
			x: 'x',
			y: 'LEAKED'
		})
		expect(evaluated).not.toContain('LEAKED')
		expect(evaluated).toContain('` + flow_input.y + `')
	})

	// An expression the old blanket rule broke — it escaped backticks inside `${...}`, which
	// does not parse — comes back as the author typed it, instead of showing the backslashes
	// and escaping them one deeper on every save.
	it('heals an expression the old rule broke', () => {
		const broken = '-p ${a}${b ? \\` --x ${c}\\` : ""}'
		const clean = '-p ${a}${b ? ` --x ${c}` : ""}'
		expect(unescapeTemplateBackticks(broken)).toBe(clean)
		expect(escapeTemplateBackticks(clean)).toBe(clean)
	})

	// A text that already parses is left alone: an over-escaped legacy value and a backslash the
	// author wrote are the same bytes, so healing on looks alone would drop a real character.
	it('leaves an expression that already parses alone', () => {
		const run = (body: string) => new Function('return `' + body + '`')()
		for (const stored of ['${"\\`"}', '${"a\\\\`"}']) {
			expect(unescapeTemplateBackticks(stored)).toBe(stored)
			expect(run(escapeTemplateBackticks(unescapeTemplateBackticks(stored)))).toBe(run(stored))
		}
	})

	// ...but a backtick escaped inside a nested template belongs there and must survive.
	it('leaves an escaped backtick that is inside a nested template alone', () => {
		const stored = '${cond ? `a\\`b` : ""}'
		expect(unescapeTemplateBackticks(stored)).toBe(stored)
	})

	// Escapes the author wrote inside a nested template are not the old rule's doing, and
	// stripping them changes what the expression means — here into chained tagged templates,
	// which throw. Only a text whose backticks are *all* escaped came from the old rule.
	it('leaves escapes that belong to a nested template alone', () => {
		const run = (body: string) => new Function('flag', 'value', 'return `' + body + '`')(true, 'X')
		for (const stored of ['${flag ? `\\`\\`${value}\\`\\`` : ""}', '${flag ? `a\\`b` : ""}']) {
			expect(unescapeTemplateBackticks(stored)).toBe(stored)
			expect(escapeTemplateBackticks(unescapeTemplateBackticks(stored))).toBe(stored)
			expect(run(stored)).toBe(run(escapeTemplateBackticks(unescapeTemplateBackticks(stored))))
		}
	})

	it('round-trips through unescapeTemplateBackticks', () => {
		for (const v of [
			nested,
			'a ` b',
			'${ x["}"] } `',
			'plain',
			'${a}${b}',
			'\\${a}',
			'${cond ? `a\\`b` : ""}'
		]) {
			expect(unescapeTemplateBackticks(escapeTemplateBackticks(v))).toBe(v)
		}
	})
	// The guarantee that matters: opening a flow in the editor and saving it back must not change
	// the stored expression, including for a value that only the all-or-nothing fallback can
	// handle.
	it('never rewrites the stored expression on a view/save cycle', () => {
		const inputs = [
			'{"match": ${/{/.test(flow_input.x)}, "literal": "`x`"}',
			'${cond ? `a\\`b` : ""}',
			"${ x.replace(/'/g, '') } `",
			'a ` b',
			'${ /* ` */ x } `'
		]
		for (const v of inputs) {
			const stored = escapeTemplateBackticks(v)
			expect(() => new Function('return `' + stored + '`')).not.toThrow()
			expect(escapeTemplateBackticks(unescapeTemplateBackticks(stored))).toBe(stored)
		}
	})
})

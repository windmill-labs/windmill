import { describe, expect, it } from 'vitest'
import { computeShow, evalSandboxed, quickjsReady } from './quickjsEval.svelte'

describe('quickjsEval', () => {
	it('answers plain lookups and comparisons without the engine', () => {
		const results = { 'my step': { rows: [1, 2] }, a: { mode: 'x' } }
		expect(computeShow('fields.mode === "advanced"', { mode: 'advanced' })).toBe(true)
		expect(computeShow('return fields.n != 0', { n: 0 })).toBe(false)
		expect(evalSandboxed('results["my step"].rows', { results })).toBe(results['my step'].rows)
		expect(evalSandboxed('results.a.mode == "x"', { results })).toBe(true)
		const started = performance.now()
		expect(computeShow('fields' + ' '.repeat(30_000) + '!', {})).toBeUndefined()
		expect(performance.now() - started).toBeLessThan(50)
	})

	it('decides nothing until the engine is in', async () => {
		expect(computeShow('fields.a + 1 === 2', { a: 1 })).toBeUndefined()
		expect(computeShow('fields.constructor', { a: 1 })).toBeUndefined()
		await quickjsReady()
		expect(computeShow('fields.a + 1 === 2', { a: 1 })).toBe(true)
		expect(computeShow('fields.x === "\\x41"', { x: 'A' })).toBe(true)
		expect(computeShow('fields["\\x41"] === 1', { A: 1 })).toBe(true)
		expect(computeShow('fields.', {})).toBe(true)
		expect(computeShow('fields.n === 01', { n: 2 })).toBe(true)
	})

	it('reaches nothing of the page and stops runaway expressions', async () => {
		await quickjsReady()
		const scope = { fields: {} }
		expect(evalSandboxed('typeof window + typeof document + typeof fetch', scope)).toBe(
			'undefinedundefinedundefined'
		)
		expect(() => evalSandboxed('(() => { while (true) {} })()', scope)).toThrow()
		expect(evalSandboxed('1 + 1', scope)).toBe(2)
		const rows = Array.from({ length: 110_000 }, (_, i) => `item-${i}-xxxxxx`)
		const started = performance.now()
		expect(() =>
			evalSandboxed('(() => { for (let i = 0; i < 300; i++) __entry("r", "rows") })()', {
				r: { rows }
			})
		).toThrow()
		expect(performance.now() - started).toBeLessThan(3000)
	})

	it('copies in only the entries an expression reads', async () => {
		await quickjsReady()
		const results = {
			a: { x: 1 },
			b: { y: 2 },
			big: { rows: Array.from({ length: 300_000 }, (_, i) => ({ i, s: 'xxxxxxxxxx' })) }
		}
		expect(evalSandboxed('`${results.a.x}`', { results })).toBe('1')
		expect(evalSandboxed('((k) => results[k].y)("b")', { results })).toBe(2)
		expect(evalSandboxed('Object.keys(results).length', { results })).toBe(3)
		expect(evalSandboxed('({ ...small }).b.y', { small: { a: results.a, b: results.b } })).toBe(2)
		expect(evalSandboxed('results.hasOwnProperty("a") && "a" in results', { results })).toBe(true)
		expect(evalSandboxed('JSON.stringify({ a: results.a, b: results.b })', { results })).toBe(
			'{"a":{"x":1},"b":{"y":2}}'
		)
		expect(() => evalSandboxed('results.big.rows.length + 0', { results })).toThrow(
			/results.big is over 5 MB/
		)
		expect(() => evalSandboxed('(results.a = 1)', { results })).toThrow()
		expect(evalSandboxed('"gpt-" + 4', { flow_input: undefined, results: undefined })).toBe('gpt-4')
		expect(
			evalSandboxed('fields.optional ?? "fallback"', { fields: { optional: undefined } })
		).toBe('fallback')
	})

	it('stands in for large text only where the answer is a yes or no', async () => {
		await quickjsReady()
		const file = 'x'.repeat(2 * 2 ** 20)
		expect(computeShow('fields.file.tooLarge && fields.file.length > 0', { file })).toBe(true)
		expect(evalSandboxed('flow_input.file || ""', { flow_input: { file } })).toBe(file)
		const huge = 'x'.repeat(6 * 2 ** 20)
		expect(() => evalSandboxed('flow_input.file || ""', { flow_input: { file: huge } })).toThrow(
			/flow_input.file is over 5 MB/
		)
	})
})

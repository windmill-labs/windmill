import { describe, expect, it } from 'vitest'
import { buildFlowTree, shapeFlowRunTree, type FlowResultEntry } from './flowRunTree'

let uuidCounter = 0
function entry(partial: Partial<FlowResultEntry>): FlowResultEntry {
	uuidCounter++
	return {
		job_id: `00000000-0000-0000-0000-${String(uuidCounter).padStart(12, '0')}`,
		label: partial.step_path ? `Step ${partial.step_path}` : 'Flow',
		kind: 'script',
		depth: 1,
		sibling_index: 1,
		sibling_count: 1,
		status: 'success',
		success: true,
		result_prefix: '"ok"',
		result_length: 4,
		...partial
	}
}

function root(partial: Partial<FlowResultEntry> = {}): FlowResultEntry {
	return entry({ kind: 'flow', depth: 0, step_path: null, label: 'Flow', ...partial })
}

/** Root flow with steps a, b where b is a subflow containing c. */
function nestedEntries(): FlowResultEntry[] {
	return [
		root(),
		entry({ step_path: 'a', flow_step_id: 'a' }),
		entry({ step_path: 'b', flow_step_id: 'b', kind: 'flow', label: 'Step b (subflow)' }),
		entry({
			step_path: 'b/c',
			flow_step_id: 'c',
			depth: 2,
			status: 'failure',
			success: false,
			result_prefix: '{"error":"boom"}',
			result_length: 16
		})
	]
}

/** Root flow with loop step l of `n` iterations; `failed` are 1-based indices. */
function loopEntries(n: number, failed: number[]): FlowResultEntry[] {
	const entries = [root()]
	for (let i = 1; i <= n; i++) {
		entries.push(
			entry({
				step_path: 'l',
				flow_step_id: 'l',
				kind: 'flow',
				label: `Step l forloop (iteration ${i}/${n})`,
				parent_module_type: 'forloopflow',
				sibling_index: i,
				sibling_count: n,
				status: failed.includes(i) ? 'failure' : 'success',
				success: !failed.includes(i)
			})
		)
	}
	return entries
}

describe('buildFlowTree', () => {
	it('rebuilds nesting from the depth-first flat list', () => {
		const tree = buildFlowTree(nestedEntries())!
		expect(tree.children.map((c) => c.entry.flow_step_id)).toEqual(['a', 'b'])
		expect(tree.children[1].children.map((c) => c.entry.flow_step_id)).toEqual(['c'])
	})
})

describe('shapeFlowRunTree', () => {
	it('renders subflow steps nested under their parent step', () => {
		const rendered = JSON.parse(shapeFlowRunTree({ entries: nestedEntries() }))
		expect(rendered.run.status).toBe('success')
		const stepB = rendered.steps[1]
		expect(stepB.step).toBe('b')
		expect(stepB.steps[0].step).toBe('b/c')
		expect(stepB.steps[0].status).toBe('failure')
		expect(stepB.steps[0].result).toBe('{"error":"boom"}')
	})

	it('collapses loop iterations to a tally with capped failed iterations plus the last', () => {
		const rendered = JSON.parse(shapeFlowRunTree({ entries: loopEntries(50, [3, 7, 21, 30]) }))
		const loop = rendered.steps[0]
		expect(loop.type).toBe('forloopflow')
		expect(loop.iterations).toBe(50)
		expect(loop.ok).toBe(46)
		expect(loop.failed_iterations).toEqual([3, 7, 21, 30])
		// 3 failed shown (cap) + the last iteration
		expect(loop.iterations_shown.map((i: any) => i.iteration)).toEqual([3, 7, 21, 50])
		expect(loop.iterations_elided).toBe(46)
	})

	it('renders retried steps as attempts of one step, not loop iterations', () => {
		const entries = [root()]
		for (let i = 1; i <= 3; i++) {
			entries.push(
				entry({
					step_path: 'a',
					flow_step_id: 'a',
					label: `Step a (attempt ${i}/3)`,
					parent_module_type: 'rawscript',
					sibling_index: i,
					sibling_count: 3,
					status: i < 3 ? 'failure' : 'success',
					success: i === 3
				})
			)
		}
		const step = JSON.parse(shapeFlowRunTree({ entries })).steps[0]
		expect(step.iterations).toBeUndefined()
		expect(step.attempts).toBe(3)
		expect(step.status).toBe('success')
		expect(step.label).toBe('Step a (attempt 3/3)')
		expect(step.previous_attempts.map((a: any) => a.status)).toEqual(['failure', 'failure'])
	})

	it('counts running iterations as unfinished and skipped ones as skipped', () => {
		const entries = loopEntries(4, [])
		entries[2].status = 'running'
		entries[2].success = undefined
		entries[2].result_prefix = undefined
		entries[2].result_length = undefined
		entries[3].status = 'skipped'
		const loop = JSON.parse(shapeFlowRunTree({ entries })).steps[0]
		expect(loop.ok).toBe(2)
		expect(loop.unfinished).toBe(1)
		expect(loop.skipped).toBe(1)
		expect(loop.failed_iterations).toBeUndefined()
	})

	it('reports the truncated total size and surfaces the enclosing flow of a step job', () => {
		const entries = nestedEntries()
		entries[1].result_prefix = 'x'.repeat(700)
		entries[1].result_length = 5000
		const rendered = JSON.parse(shapeFlowRunTree({ enclosing_job: 'enclosing-uuid', entries }))
		expect(rendered.note).toContain('enclosing-uuid')
		expect(rendered.steps[0].result_total_chars).toBe(5000)
	})

	it('notes when the server truncated the tree', () => {
		const rendered = JSON.parse(shapeFlowRunTree({ entries: nestedEntries(), truncated: true }))
		expect(rendered.note).toContain('more jobs')
	})

	it('compares result sizes in code points and never splits a surrogate pair', () => {
		const entries = [root()]
		// 400 astral chars: 800 UTF-16 units but 400 code points. The 700-unit
		// head budget keeps only 350 of them — comparing in UTF-16 units
		// (400 < 700) used to hide that truncation entirely.
		entries.push(
			entry({
				step_path: 'a',
				flow_step_id: 'a',
				status: 'failure',
				success: false,
				result_prefix: '🦄'.repeat(400),
				result_length: 400
			})
		)
		const step = JSON.parse(shapeFlowRunTree({ entries })).steps[0]
		expect(step.result_total_chars).toBe(400)
		// the cut must never leave a lone high surrogate at the tail
		expect(step.result.charCodeAt(step.result.length - 1)).toBeGreaterThan(0xdbff)
	})

	it('shrinks result heads to fit the total budget instead of overflowing', () => {
		const entries = [root()]
		for (let i = 1; i <= 60; i++) {
			entries.push(
				entry({
					step_path: `s${i}`,
					flow_step_id: `s${i}`,
					status: i === 1 ? 'failure' : 'success',
					success: i !== 1,
					result_prefix: 'y'.repeat(700),
					result_length: 700
				})
			)
		}
		const rendered = shapeFlowRunTree({ entries })
		expect(rendered.length).toBeLessThanOrEqual(21000)
		const parsed = JSON.parse(rendered)
		// failure detail outlives success detail: the failed step keeps a result
		// head while succeeded steps are reduced to their size
		expect(parsed.steps[0].status).toBe('failure')
		expect(parsed.steps[0].result.length).toBeGreaterThan(0)
		expect(parsed.steps[1].result).toBeUndefined()
		expect(parsed.steps[1].result_total_chars).toBe(700)
	})
})

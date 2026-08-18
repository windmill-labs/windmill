import { describe, expect, it } from 'vitest'
import {
	autoOutputAsset,
	compatibleOutputKinds,
	generatePipelineDraft,
	PIPELINE_OUTPUT_KINDS
} from './pipelineTemplates'

// The `data_test` output kind scaffolds a *custom* (singular) data test: a
// standalone DuckDB script referenced from a materialize script's
// `-- data_test <path>` line. It must be a single SELECT that reads the
// freshly-materialized target through the internal `_wm_target` schema — the
// two rules the backend's self-teaching errors enforce.
describe('data_test scaffold', () => {
	it('is a DuckDB-only output kind exposed in the picker', () => {
		expect(compatibleOutputKinds('duckdb')).toContain('data_test')
		expect(compatibleOutputKinds('python3')).not.toContain('data_test')
		expect(PIPELINE_OUTPUT_KINDS.map((k) => k.id)).toContain('data_test')
	})

	it('produces no output asset (it asserts against an existing target)', () => {
		expect(autoOutputAsset('data_test', 'folder', 'duckdb')).toBeUndefined()
	})

	it('scaffolds a single SELECT against `_wm_target.<table>`', () => {
		const src = generatePipelineDraft({
			language: 'duckdb',
			outputKind: 'data_test',
			triggers: []
		})
		// starter body is a single SELECT against the internal target alias.
		expect(src).toContain('SELECT * FROM _wm_target.your_table WHERE your_condition;')
		// exactly one SQL statement (single SELECT) — count statement lines, not
		// the word "SELECT" that also appears in the guidance comment.
		const stmtLines = src.split('\n').filter((l) => /^\s*SELECT\b/i.test(l))
		expect(stmtLines).toHaveLength(1)
		// no `-- materialize <uri>` output annotation — a data test declares no
		// asset (the word still appears in the guidance comment, which is fine).
		expect(src).not.toMatch(/^--\s*materialize\s/m)
		// teaches how to wire it up + the offending-rows convention.
		expect(src).toContain('-- data_test <this-script-path>')
		expect(src).toContain('offending rows')
	})

	it('seeds the table name from an upstream ducklake asset when present', () => {
		const src = generatePipelineDraft({
			language: 'duckdb',
			outputKind: 'data_test',
			input: { kind: 'ducklake', path: 'analytics/orders' },
			triggers: []
		})
		expect(src).toContain('SELECT * FROM _wm_target.orders WHERE your_condition;')
		// no ATTACH of the input — the runtime attaches the target as `_wm_target`.
		expect(src).not.toContain('ATTACH')
	})
})

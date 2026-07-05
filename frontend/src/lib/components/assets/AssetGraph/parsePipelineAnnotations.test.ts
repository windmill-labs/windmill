import { describe, expect, it } from 'vitest'
import {
	mergeColumnLineage,
	parseDurationSecs,
	parsePipelineAnnotations,
	type ColumnLineage
} from './parsePipelineAnnotations'

// Unit-test the TS mirror of the backend `parse_pipeline_annotations`. The
// two implementations MUST stay behaviorally identical — these tests are
// intentionally parallel to the Rust ones in asset_parser.rs.

describe('parsePipelineAnnotations: tag', () => {
	it('parses a basic tag', () => {
		const out = parsePipelineAnnotations('// tag heavy')
		expect(out.tag).toBe('heavy')
	})

	it('first declaration wins', () => {
		const out = parsePipelineAnnotations('// tag heavy\n# tag light')
		expect(out.tag).toBe('heavy')
	})

	it('skips empty tag', () => {
		const out = parsePipelineAnnotations('// tag   ')
		expect(out.tag).toBeUndefined()
	})

	it('requires keyword to be a whole word', () => {
		const out = parsePipelineAnnotations('// tagged heavy')
		expect(out.tag).toBeUndefined()
	})

	it('skips a tag value containing whitespace (regular comment false-positive)', () => {
		const out = parsePipelineAnnotations('# tag this function so we remember to refactor it later')
		expect(out.tag).toBeUndefined()
	})

	it('skips a tag value longer than 50 chars', () => {
		const out = parsePipelineAnnotations('// tag ' + 'x'.repeat(51))
		expect(out.tag).toBeUndefined()
	})
})

describe('parsePipelineAnnotations: header scan', () => {
	it('ignores annotations in the body once code has started', () => {
		const code = [
			'import pandas as pd',
			'',
			'def main():',
			'    # tag each row with its source so downstream steps can filter',
			'    # on s3://should/not/parse',
			'    return pd.DataFrame()'
		].join('\n')
		const out = parsePipelineAnnotations(code)
		expect(out.tag).toBeUndefined()
		expect(out.triggerAssets).toHaveLength(0)
	})

	it('tolerates blank lines before code but stops at the first code line', () => {
		const code = ['#!/usr/bin/env python', '', '# tag heavy', 'import os', '# tag light'].join('\n')
		const out = parsePipelineAnnotations(code)
		expect(out.tag).toBe('heavy')
	})
})

describe('parsePipelineAnnotations: retry', () => {
	it('parses count only', () => {
		const out = parsePipelineAnnotations('// retry 3')
		expect(out.retry).toEqual({ count: 3 })
	})

	it('parses count + delay', () => {
		const out = parsePipelineAnnotations('// retry 3 5s')
		expect(out.retry).toEqual({ count: 3, delay: '5s' })
	})

	it('first declaration wins', () => {
		const out = parsePipelineAnnotations('// retry 3 5s\n# retry 1')
		expect(out.retry).toEqual({ count: 3, delay: '5s' })
	})

	it('rejects zero count', () => {
		const out = parsePipelineAnnotations('// retry 0 5s')
		expect(out.retry).toBeUndefined()
	})

	it('rejects non-numeric count', () => {
		const out = parsePipelineAnnotations('// retry many')
		expect(out.retry).toBeUndefined()
	})

	it('rejects partial-numeric count (matches backend u32::parse strictness)', () => {
		const out = parsePipelineAnnotations('// retry 3foo')
		expect(out.retry).toBeUndefined()
	})
})

describe('parsePipelineAnnotations: macros + use', () => {
	it('parses the bare macros marker', () => {
		const out = parsePipelineAnnotations('// macros\nCREATE MACRO m(a) AS a;')
		expect(out.macros).toBe(true)
	})

	it('macros marker is strict — trailing prose and variants rejected', () => {
		expect(parsePipelineAnnotations('// macros are defined below\n').macros).toBe(false)
		expect(parsePipelineAnnotations('// macros_v2\n').macros).toBe(false)
		expect(parsePipelineAnnotations('-- macros   \nSELECT 1;').macros).toBe(true)
	})

	it('macros wins over pipeline — a library is never a pipeline member', () => {
		const out = parsePipelineAnnotations('// pipeline\n// macros\nCREATE MACRO m(a) AS a;')
		expect(out.macros).toBe(true)
		expect(out.inPipeline).toBe(false)
	})

	it('use accumulates in order and dedups', () => {
		const out = parsePipelineAnnotations(
			'// use f/lib/stats\n// use f/lib/dates\n// use f/lib/stats\n'
		)
		expect(out.useLibs).toEqual(['f/lib/stats', 'f/lib/dates'])
	})

	it('use rejects prose, slashless and multi-token values', () => {
		const out = parsePipelineAnnotations(
			'// use this script to compute\n// use standalone\n// use f/lib/ok extra\n'
		)
		expect(out.useLibs).toEqual([])
	})
})

describe('parsePipelineAnnotations: combined', () => {
	it('parses all keywords together', () => {
		const code = [
			'// pipeline',
			'// on schedule',
			'// on s3://in.csv',
			'// partitioned daily tz="UTC"',
			'// freshness 2h',
			'// tag heavy',
			'// retry 3 5s'
		].join('\n')
		const out = parsePipelineAnnotations(code)
		expect(out.inPipeline).toBe(true)
		// `// on schedule` joins the native-trigger family — marker-only,
		// binding lives on the schedule row's `script_path`.
		expect(out.nativeTriggers).toEqual([{ kind: 'schedule' }])
		expect(out.triggerAssets).toHaveLength(1)
		expect(out.partition).toBeDefined()
		expect(out.freshness).toEqual({ duration: '2h' })
		expect(out.tag).toBe('heavy')
		expect(out.retry).toEqual({ count: 3, delay: '5s' })
	})
})

describe('parsePipelineAnnotations: data_upload', () => {
	it('parses the marker', () => {
		const out = parsePipelineAnnotations('// on data_upload')
		expect(out.nativeTriggers).toEqual([{ kind: 'data_upload' }])
	})

	it('rejects trailing content (marker-only)', () => {
		const out = parsePipelineAnnotations('// on data_upload f/foo')
		expect(out.nativeTriggers).toEqual([])
	})

	it('requires a whole word', () => {
		const out = parsePipelineAnnotations('// on data_uploadish')
		expect(out.nativeTriggers).toEqual([])
	})
})

describe('mergeColumnLineage', () => {
	const ref = (path: string, col: string): ColumnLineage['inputs'][number] => ({
		from_kind: 'ducklake',
		from_path: path,
		from_column: col
	})

	it('annotation wins per output column; inferred fills the rest (mirrors Rust)', () => {
		const inferred: ColumnLineage[] = [
			{ column: 'total', inputs: [ref('w/o', 'amount')] },
			{ column: 'qty', inputs: [ref('w/o', 'qty')] }
		]
		const annotated: ColumnLineage[] = [{ column: 'total', inputs: [ref('w/manual', 'grand')] }]
		const merged = mergeColumnLineage(inferred, annotated)
		expect(merged).toEqual([
			{ column: 'total', inputs: [ref('w/manual', 'grand')] }, // annotation, first + authoritative
			{ column: 'qty', inputs: [ref('w/o', 'qty')] } // inferred, not overridden
		])
	})

	it('returns annotations unchanged when there is no inferred lineage', () => {
		const annotated: ColumnLineage[] = [{ column: 'a', inputs: [ref('w/o', 'a')] }]
		expect(mergeColumnLineage([], annotated)).toEqual(annotated)
	})
})

// Mirror of the Rust `parse_duration_secs` tests (windmill-common assets.rs)
// — the freshness chip's staleness verdict depends on identical parsing.
describe('parseDurationSecs', () => {
	it('parses suffixed durations', () => {
		expect(parseDurationSecs('30s')).toBe(30)
		expect(parseDurationSecs('5m')).toBe(300)
		expect(parseDurationSecs('2h')).toBe(7200)
		expect(parseDurationSecs('1d')).toBe(86400)
	})

	it('bare integer means seconds', () => {
		expect(parseDurationSecs('45')).toBe(45)
	})

	it('tolerates surrounding whitespace', () => {
		expect(parseDurationSecs(' 5 m ')).toBe(300)
	})

	it('accepts an explicit plus sign (Rust i64 parsing does)', () => {
		expect(parseDurationSecs('+5m')).toBe(300)
		expect(parseDurationSecs('+45')).toBe(45)
	})

	it('rejects malformed / non-positive input', () => {
		expect(parseDurationSecs('')).toBeUndefined()
		expect(parseDurationSecs('h')).toBeUndefined()
		expect(parseDurationSecs('1.5h')).toBeUndefined()
		expect(parseDurationSecs('-5m')).toBeUndefined()
		expect(parseDurationSecs('0')).toBeUndefined()
		expect(parseDurationSecs('fast')).toBeUndefined()
	})

	it('rejects values beyond i32 seconds (mirrors backend cap)', () => {
		expect(parseDurationSecs('999999999d')).toBeUndefined()
	})
})

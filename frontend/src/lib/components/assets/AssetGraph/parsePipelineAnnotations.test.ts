import { describe, expect, it } from 'vitest'
import { parsePipelineAnnotations } from './parsePipelineAnnotations'

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

describe('parsePipelineAnnotations: combined', () => {
	it('parses all keywords together', () => {
		const code = [
			'// pipeline',
			'// schedule "0 0 * * *"',
			'// on s3://in.csv',
			'// partitioned daily tz="UTC"',
			'// freshness 2h',
			'// tag heavy',
			'// retry 3 5s'
		].join('\n')
		const out = parsePipelineAnnotations(code)
		expect(out.inPipeline).toBe(true)
		expect(out.schedules).toEqual(['0 0 * * *'])
		expect(out.triggerAssets).toHaveLength(1)
		expect(out.partition).toBeDefined()
		expect(out.freshness).toEqual({ duration: '2h' })
		expect(out.tag).toBe('heavy')
		expect(out.retry).toEqual({ count: 3, delay: '5s' })
	})
})

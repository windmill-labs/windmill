import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { parsePipelineAnnotations } from './parsePipelineAnnotations'

// Parser-parity guard: this TS parser (drives the live graph preview) and
// the Rust `parse_pipeline_annotations` (drives deploy) must stay
// behaviorally identical, or the graph the user previews is not the graph
// that deploys. Both run the SAME fixture corpus, owned by the Rust crate:
//
//   backend/parsers/windmill-parser/tests/fixtures/pipeline_annotations.json
//
// Rust counterpart: backend/parsers/windmill-parser/tests/
// pipeline_annotations_parity.rs. Extend the corpus when the grammar
// changes; a fixture passing on one side and failing on the other is
// exactly the drift this exists to catch. Only fields both parsers produce
// are compared (join_mode / debounce_default are deploy-only, Rust-side).

type Fixture = {
	name: string
	code: string
	expected: {
		in_pipeline: boolean
		asset_triggers: string[]
		native_triggers: string[]
		partition: {
			kind: string
			key?: string
			tz: string | null
			format: string | null
			start: string | null
		} | null
		freshness: string | null
		tag: string | null
		retry: { count: number; delay: string | null } | null
	}
}

const fixturesPath = resolve(
	dirname(fileURLToPath(import.meta.url)),
	'../../../../../../backend/parsers/windmill-parser/tests/fixtures/pipeline_annotations.json'
)
const fixtures: Fixture[] = JSON.parse(readFileSync(fixturesPath, 'utf-8'))

describe('parsePipelineAnnotations matches the shared Rust fixture corpus', () => {
	it('corpus is non-empty', () => {
		expect(fixtures.length).toBeGreaterThan(0)
	})

	for (const f of fixtures) {
		it(f.name, () => {
			const got = parsePipelineAnnotations(f.code)

			expect(got.inPipeline, 'in_pipeline').toBe(f.expected.in_pipeline)

			expect(
				got.triggerAssets.map((a) => `${a.kind}:${a.path}`),
				'asset triggers'
			).toEqual(f.expected.asset_triggers)

			expect(
				got.nativeTriggers.map((n) => n.kind),
				'native triggers'
			).toEqual(f.expected.native_triggers)

			if (f.expected.partition === null) {
				expect(got.partition, 'partition').toBeUndefined()
			} else {
				expect(got.partition?.kind, 'partition kind').toBe(f.expected.partition.kind)
				expect(
					got.partition && 'key' in got.partition ? got.partition.key : undefined,
					'partition key'
				).toEqual(f.expected.partition.key ?? undefined)
				expect(got.partition?.tz, 'partition tz').toEqual(f.expected.partition.tz ?? undefined)
				expect(got.partition?.format, 'partition format').toEqual(
					f.expected.partition.format ?? undefined
				)
				expect(got.partition?.start, 'partition start').toEqual(
					f.expected.partition.start ?? undefined
				)
			}

			expect(got.freshness?.duration, 'freshness').toEqual(f.expected.freshness ?? undefined)
			expect(got.tag, 'tag').toEqual(f.expected.tag ?? undefined)

			if (f.expected.retry === null) {
				expect(got.retry, 'retry').toBeUndefined()
			} else {
				expect(got.retry?.count, 'retry count').toBe(f.expected.retry.count)
				expect(got.retry?.delay, 'retry delay').toEqual(f.expected.retry.delay ?? undefined)
			}
		})
	}
})

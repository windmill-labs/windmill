import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'

// Every field the TS parser produces, each with an assertion in the per-fixture
// test below. Typed as `Record<keyof PipelineAnnotations, true>` so adding a
// field to `PipelineAnnotations` is a compile error here until it's listed —
// forcing the author to also wire up its parity assertion. (Deploy-only Rust
// fields like join_mode / debounce_default are not part of this type, so they
// correctly never appear here.)
const ASSERTED_TS_FIELDS: Record<keyof PipelineAnnotations, true> = {
	inPipeline: true,
	triggerAssets: true,
	nativeTriggers: true,
	partition: true,
	freshness: true,
	tag: true,
	retry: true,
	materialize: true
}

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
// exactly the drift this exists to catch.
//
// Intentional divergence — the Rust parser is a superset. It also parses
// `join_mode` and `debounce_default`, which are DEPLOY-ONLY: they affect how
// the backend schedules cascade runs, never the rendered graph, so the TS
// preview parser deliberately doesn't produce them and they are not compared
// here. Every field the TS parser DOES produce is compared, and the
// `ASSERTED_TS_FIELDS` exhaustiveness check above fails the suite if a new TS
// field is added without a matching assertion — so a field can't be parsed on
// the TS side yet silently skipped by this guard.

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
		materialize?: {
			target_kind: string
			target_path: string
			manual?: boolean
			append?: boolean
			unique_key?: string | null
		} | null
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

	it('every field the parser emits across the corpus has a parity assertion', () => {
		const asserted = new Set(Object.keys(ASSERTED_TS_FIELDS))
		const emitted = new Set<string>()
		for (const f of fixtures) {
			for (const k of Object.keys(parsePipelineAnnotations(f.code))) emitted.add(k)
		}
		expect(
			[...emitted].filter((k) => !asserted.has(k)),
			'unasserted parser fields'
		).toEqual([])
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

			if (f.expected.materialize == null) {
				expect(got.materialize, 'materialize').toBeUndefined()
			} else {
				expect(got.materialize?.targetKind, 'materialize target kind').toBe(
					f.expected.materialize.target_kind
				)
				expect(got.materialize?.targetPath, 'materialize target path').toBe(
					f.expected.materialize.target_path
				)
				expect(got.materialize?.manual ?? false, 'materialize manual').toBe(
					f.expected.materialize.manual ?? false
				)
				expect(got.materialize?.append ?? false, 'materialize append').toBe(
					f.expected.materialize.append ?? false
				)
				expect(got.materialize?.uniqueKey, 'materialize key').toEqual(
					f.expected.materialize.unique_key ?? undefined
				)
			}
		})
	}
})

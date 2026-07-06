import { describe, expect, it } from 'vitest'
import type { ScriptLang } from '$lib/gen'
import {
	autoOutputAsset,
	generatePipelineDraft,
	type PipelineOutputKind
} from './pipelineTemplates'

// The seeded draft asset (`autoOutputAsset`, stored as `outputAssets` and used
// by resolveGraph for inactive-draft node identity) must match the asset
// identity the deploy-time / wasm parser infers from the generated body. The
// parser canonicalizes any S3 URI by stripping the `s3://` prefix and all
// leading slashes (see backend `parse_asset_syntax`); if the seed carried a
// leading slash while the body wrote `s3:///key`, the preview would render a
// duplicate `/key` node and a phantom post-deploy drift. This pins the two in
// lockstep so that class of drift can't regress.

// Mirror of the parser's S3 canonicalization for a raw `s3://…` URI.
function canonicalS3Key(uri: string): string {
	const rest = uri.replace(/^s3:\/\//, '')
	return rest.replace(/^\/+/, '')
}

const S3_KINDS: PipelineOutputKind[] = ['s3_parquet', 's3_object']
const LANGS: ScriptLang[] = ['bun', 'python3', 'duckdb']

describe('pipelineTemplates S3 seed/body parity', () => {
	for (const language of LANGS) {
		for (const outputKind of S3_KINDS) {
			it(`${language} ${outputKind}: seeded asset path matches the body's S3 write URI`, () => {
				const output = autoOutputAsset(outputKind, 'demo', language)
				expect(output).toBeDefined()
				const asset = output!

				// The seed must be a canonical slashless key so it matches the
				// identity the parser infers from the generated body.
				expect(asset.kind).toBe('s3object')
				expect(asset.path.startsWith('/')).toBe(false)

				const body = generatePipelineDraft({
					language,
					outputKind,
					output: asset,
					triggers: []
				})

				// Every S3 URI the generated body emits must canonicalize back to
				// the seeded asset path — the write target especially.
				const uris = body.match(/s3:\/\/[^'"`)\s]+/g) ?? []
				expect(uris.length).toBeGreaterThan(0)
				for (const uri of uris) {
					// Runtime form must stay triple-slash (default storage); a bare
					// `s3://key` would target a named storage `key` at run time.
					expect(uri.startsWith('s3:///')).toBe(true)
					expect(canonicalS3Key(uri)).toBe(asset.path)
				}
			})
		}
	}
})

// The `{partition}` token substitutes to the partition IDENTITY string (e.g.
// `2026-07-05T23`, `2026-W27`, `2026-07`), which is NOT a valid DuckDB
// TIMESTAMP literal for any sub-day / non-daily grain — so a naive
// `WHERE ts = TIMESTAMP {partition}` raises a Conversion Error for hourly/
// weekly/monthly (only daily happens to parse). The scaffold must teach the
// grain-agnostic `strftime(<ts_col>, '<fmt>') = {partition}` idiom instead, so
// a user who adds `-- partitioned hourly` never hits that footgun.
describe('pipelineTemplates materialize partition filter', () => {
	const materializeBody = () =>
		generatePipelineDraft({
			language: 'duckdb',
			outputKind: 'materialize',
			output: autoOutputAsset('materialize', 'demo', 'duckdb'),
			input: { kind: 'ducklake', path: 'main/orders' },
			triggers: []
		})

	it('teaches the strftime idiom for the hourly grain', () => {
		const body = materializeBody()
		// The exact hourly format the runtime builds the identity from — a
		// `TIMESTAMP {partition}` cast on this string errors at runtime.
		expect(body).toContain('strftime')
		expect(body).toContain(`strftime(<ts_col>, '%Y-%m-%dT%H') = {partition}`)
	})

	it('covers weekly and monthly grains too', () => {
		const body = materializeBody()
		expect(body).toContain(`strftime(<ts_col>, '%G-W%V')`) // weekly
		expect(body).toContain(`strftime(<ts_col>, '%Y-%m')`) // monthly
		expect(body).toContain(`strftime(<ts_col>, '%Y-%m-%d')`) // daily
	})

	it('never scaffolds the naive `TIMESTAMP {partition}` cast as executable SQL', () => {
		// The footgun only parses for daily and Conversion-Errors for every
		// finer/coarser grain. It may appear in an "avoid it" comment, but must
		// never be emitted as an actual (non-comment) SQL line.
		const sqlLines = materializeBody()
			.split('\n')
			.filter((l) => !l.trimStart().startsWith('--'))
		for (const line of sqlLines) {
			expect(line).not.toMatch(/TIMESTAMP\s*\{partition\}/)
		}
	})
})

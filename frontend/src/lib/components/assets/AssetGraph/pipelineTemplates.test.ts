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
// parser canonicalizes any S3 URI by stripping the `s3://` prefix and a single
// leading slash (see backend `parse_asset_syntax`); if the seed carried a
// leading slash while the body wrote `s3:///key`, the preview would render a
// duplicate `/key` node and a phantom post-deploy drift. This pins the two in
// lockstep so that class of drift can't regress.

// Mirror of the parser's S3 canonicalization for a raw `s3://…` URI.
function canonicalS3Key(uri: string): string {
	const rest = uri.replace(/^s3:\/\//, '')
	return rest.replace(/^\//, '')
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

				// The seed must be a canonical slashless key (the invariant that
				// broke when the parser started stripping the leading slash).
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

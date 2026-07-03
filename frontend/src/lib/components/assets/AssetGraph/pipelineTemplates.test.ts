import { describe, expect, it } from 'vitest'
import { INGESTION_TEMPLATES } from './pipelineTemplates'

// The two-script ingestion templates carry a fragile cross-script invariant:
// the entry body's SDK landing key (bare, no leading slash), the seeded
// output asset path (leading slash), and the loader's `-- on s3:///<key>`
// URI (triple slash) must all canonicalize to the same object — otherwise
// the entry's write edge, the loader's trigger, and its body read silently
// land on different asset nodes and the lineage breaks (see
// docs/pipeline-ingestion.md §Gotchas).
describe('INGESTION_TEMPLATES landing-path invariants', () => {
	const ctx = { folder: 'demo_folder', base: 'my_ingest' }

	for (const tpl of INGESTION_TEMPLATES) {
		const scripts = tpl.generate(ctx)

		it(`${tpl.id}: every script is a pipeline member`, () => {
			for (const s of scripts) {
				expect(s.content).toMatch(/^(--|#|\/\/) pipeline\n/)
			}
		})

		if (scripts.length === 1) continue

		it(`${tpl.id}: entry, seeded output and loader agree on the landing object`, () => {
			const [entry, loader] = scripts
			expect(entry.suffix).toBe('')
			expect(loader.language).toBe('duckdb')

			// Seeded output is the canonical `/<key>` form.
			const canonical = entry.output?.path
			expect(canonical).toMatch(/^\//)
			const key = canonical!.slice(1)

			// Entry body writes the bare key via the SDK object form.
			expect(entry.content).toContain(JSON.stringify(key))

			// Loader is triggered by and reads the triple-slash URI of the
			// same key.
			expect(loader.content).toContain(`-- on s3:///${key}`)
			expect(loader.content).toContain(`read_json_auto('s3:///${key}')`)
		})

		it(`${tpl.id}: loader declares a managed materialize target`, () => {
			const loader = scripts[1]
			expect(loader.content).toMatch(/^-- materialize ducklake:\/\//m)
		})
	}
})

import { describe, it, expect } from 'vitest'
import { PipelineEditorState } from './pipelineEditorState.svelte'
import type { PipelineDraft } from './pipelineAiHelpers'
import type { AssetKind } from '$lib/gen'

// handleDraftPersist defers its commit a microtask (so a same-batch discard can
// win); flush that microtask before asserting.
const flushMicrotasks = () => new Promise<void>((resolve) => queueMicrotask(() => resolve()))

function draft(content: string, outputAssets?: { kind: AssetKind; path: string }[]): PipelineDraft {
	return {
		localId: 'pe-1',
		script: { path: 'f/x/n', language: 'duckdb', content } as PipelineDraft['script'],
		outputAssets
	}
}

describe('PipelineEditorState.handleDraftPersist', () => {
	// Regression: a no-output draft has `outputAssets: undefined`; the details pane
	// infers an empty `writes: []`. Both mean "no writes", so persisting unchanged
	// content+writes must be a no-op. The earlier `undefined === 0` length check made
	// it false, so every persist re-wrote the drafts Map with an equivalent object —
	// re-triggering the pane's emit → graph re-derive → persist, an infinite
	// microtask loop that froze the tab. The Map reference must stay identical.
	it('is idempotent for a no-output draft (undefined outputAssets vs empty inferred writes)', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([['f/x/n', draft('SELECT 1', undefined)]])
		const before = pe.drafts
		pe.handleDraftPersist('f/x/n', { content: 'SELECT 1', writes: [] })
		await flushMicrotasks()
		expect(pe.drafts).toBe(before)
	})

	it('re-writes the drafts Map when the content actually changes', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([['f/x/n', draft('SELECT 1', undefined)]])
		const before = pe.drafts
		pe.handleDraftPersist('f/x/n', { content: 'SELECT 2', writes: [] })
		await flushMicrotasks()
		expect(pe.drafts).not.toBe(before)
		expect(pe.drafts.get('f/x/n')?.script.content).toBe('SELECT 2')
	})

	it('re-writes the drafts Map when the inferred writes actually change', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([['f/x/n', draft('SELECT 1', undefined)]])
		const before = pe.drafts
		pe.handleDraftPersist('f/x/n', {
			content: 'SELECT 1',
			writes: [{ kind: 'resource' as AssetKind, path: 'f/x/out' }]
		})
		await flushMicrotasks()
		expect(pe.drafts).not.toBe(before)
		expect(pe.drafts.get('f/x/n')?.outputAssets).toEqual([{ kind: 'resource', path: 'f/x/out' }])
	})

	it('stays idempotent when outputAssets and inferred writes match (non-empty)', async () => {
		const pe = new PipelineEditorState()
		pe.drafts = new Map([
			['f/x/n', draft('SELECT 1', [{ kind: 'resource' as AssetKind, path: 'f/x/out' }])]
		])
		const before = pe.drafts
		pe.handleDraftPersist('f/x/n', {
			content: 'SELECT 1',
			writes: [{ kind: 'resource' as AssetKind, path: 'f/x/out' }]
		})
		await flushMicrotasks()
		expect(pe.drafts).toBe(before)
	})
})

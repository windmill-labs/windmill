import { describe, it, expect } from 'vitest'
import {
	runtimeRawAppToDraft,
	applyDraftToRuntimeRawApp,
	type RuntimeRawApp,
	type RawAppDraft
} from './appDraftCodec'

function runtime(over: Partial<RuntimeRawApp> = {}): RuntimeRawApp {
	return {
		summary: 'app',
		path: 'f/foo/app',
		files: { '/index.tsx': 'x' },
		runnables: {},
		data: { tables: [] } as any,
		policy: { execution_mode: 'publisher' },
		...over
	}
}

describe('appDraftCodec — custom_path round-trip', () => {
	it('carries custom_path from runtime → draft', () => {
		const draft = runtimeRawAppToDraft(runtime({ custom_path: 'my-url' }))
		expect(draft.custom_path).toBe('my-url')
	})

	it('carries custom_path from draft → runtime', () => {
		const base = runtime({ custom_path: undefined })
		const dv: RawAppDraft = {
			summary: 'app',
			files: {},
			runnables: {},
			data: { tables: [] } as any,
			policy: { execution_mode: 'publisher' },
			custom_path: 'kept-url'
		}
		expect(applyDraftToRuntimeRawApp(base, dv).custom_path).toBe('kept-url')
	})

	it('survives a full runtime → draft → runtime round-trip', () => {
		const original = runtime({ custom_path: 'round-trip-url' })
		const back = applyDraftToRuntimeRawApp(
			runtime({ custom_path: undefined }),
			runtimeRawAppToDraft(original)
		)
		expect(back.custom_path).toBe('round-trip-url')
		// runtime-only `path` is preserved from the target, not the draft
		expect(back.path).toBe('f/foo/app')
	})

	it('falls back to the runtime custom_path when the draft omits it', () => {
		const base = runtime({ custom_path: 'existing' })
		const dv: RawAppDraft = {
			summary: 'app',
			files: {},
			runnables: {},
			data: { tables: [] } as any
		}
		expect(applyDraftToRuntimeRawApp(base, dv).custom_path).toBe('existing')
	})
})

describe('appDraftCodec — draft_path round-trip', () => {
	it('serializes draft_path so a path edit changes the draft (and its sig)', () => {
		const draft = runtimeRawAppToDraft(runtime({ draft_path: 'u/admin/friendly' }))
		expect(draft.draft_path).toBe('u/admin/friendly')
		// The autosave keys on JSON.stringify(draft); without draft_path a rename
		// would be invisible and never persist.
		expect(JSON.stringify(draft)).toContain('u/admin/friendly')
	})

	it('survives a full runtime → draft → runtime round-trip', () => {
		const original = runtime({ draft_path: 'u/admin/renamed' })
		const back = applyDraftToRuntimeRawApp(
			runtime({ draft_path: undefined }),
			runtimeRawAppToDraft(original)
		)
		expect(back.draft_path).toBe('u/admin/renamed')
	})
})

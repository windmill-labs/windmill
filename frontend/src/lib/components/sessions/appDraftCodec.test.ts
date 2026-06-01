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

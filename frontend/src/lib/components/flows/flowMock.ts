import type { FlowModule } from '$lib/gen'

type Mock = NonNullable<FlowModule['mock']>

/**
 * A pin set from the editor is a test aid, so a step pinned for the first time applies to preview
 * runs only. A step that already had a mock keeps its setting: one with no `preview_only` key
 * applies to every run, and toggling it back on must not change that.
 */
export function nextMock<M extends Mock>(prev: FlowModule['mock'], next: M): M {
	return { ...next, preview_only: next.preview_only ?? (prev ? prev.preview_only : true) }
}

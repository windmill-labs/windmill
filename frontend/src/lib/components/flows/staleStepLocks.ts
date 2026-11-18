import type { FlowModule, FlowValue } from '$lib/gen'
import { forEachFlowModule } from './dfs'

/**
 * Drops, in place, the lock of every inline step whose code differs from the same step in
 * `base`. The worker reuses any non-empty step lock, so a step whose code changed but kept its
 * lock runs against the old dependencies. Unchanged steps keep theirs, so a deploy does not
 * re-resolve (and possibly upgrade) what nobody touched. A step id `base` lacks (a renamed or
 * duplicated step) keeps its lock only when some `base` step has the same code. Without a base
 * nothing is dropped: a copied or imported flow carries locks that match its code.
 */
export function dropStaleStepLocks(value: FlowValue, base: FlowValue | undefined): void {
	if (!base) return
	const allModules = (v: FlowValue) =>
		[...(v.modules ?? []), v.preprocessor_module, v.failure_module].filter(
			(m): m is FlowModule => m != null
		)
	const contentOf = (m: FlowModule) => (m.value as { content?: unknown }).content
	const baseContentById = new Map<string, unknown>()
	const baseContents = new Set<unknown>()
	forEachFlowModule(allModules(base), (m) => {
		baseContentById.set(m.id, contentOf(m))
		baseContents.add(contentOf(m))
	})
	forEachFlowModule(allModules(value), (m) => {
		const step = m.value as { content?: unknown; lock?: unknown }
		if (step.lock == undefined) return
		const unchanged = baseContentById.has(m.id)
			? baseContentById.get(m.id) === step.content
			: baseContents.has(step.content)
		if (!unchanged) delete step.lock
	})
}

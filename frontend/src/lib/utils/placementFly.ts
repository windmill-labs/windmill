import { fly, type TransitionConfig } from 'svelte/transition'

/**
 * Fly a melt-positioned floating element in from its anchor. Melt stamps the
 * resolved side (after flip/fitViewport) on `data-side`, but only once its
 * async computePosition settles — after the intro's config is read. So the
 * caller passes its requested `placement` as the intro fallback, and
 * `data-side` (read via a deferred config) takes over when present, i.e. on
 * the outro and whenever positioning already settled. Missing both falls back
 * to the common case (opens below → slides down from the trigger).
 */
export function placementFly(
	node: Element,
	{
		duration = 100,
		distance = 16,
		placement
	}: { duration?: number; distance?: number; placement?: string } = {}
): () => TransitionConfig {
	return () => {
		const side = node.getAttribute('data-side') ?? placement?.split('-')[0] ?? 'bottom'
		const x = side === 'left' ? distance : side === 'right' ? -distance : 0
		const y = side === 'top' ? distance : side === 'left' || side === 'right' ? 0 : -distance
		return fly(node, { duration, x, y })
	}
}

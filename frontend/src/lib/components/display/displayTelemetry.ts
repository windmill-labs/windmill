import { logFeatureUsage } from '$lib/utils/featureUsage'
import { fenceKind, parseFence, type FenceKind } from './displayCatalog'

const DISPLAY_BLOCK = /```wm-display[^\n]*\n([\s\S]*?)```/g

/** Counts the ```wm-display blocks of a finished assistant reply, by kind or as `invalid`. */
export function logDisplayBlocks(reply: string): void {
	if (!reply.includes('```wm-display')) return
	for (const [, source] of reply.matchAll(DISPLAY_BLOCK)) {
		const parsed = parseFence(source)
		const key: FenceKind | 'invalid' = parsed.ok ? fenceKind(parsed.result) : 'invalid'
		logFeatureUsage('ai_chat', 'display_block', { key })
	}
}

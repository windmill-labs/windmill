import { logFeatureUsage } from '$lib/utils/featureUsage'

// Anonymous counters for the reusable-agent lifecycle (`docs/reusable-ai-agents.md`). Same rules
// as every other `logFeatureUsage` caller: aggregated counts only, and the four keys below are
// the whole vocabulary — no agent path, prompt, model or tool ever reaches here.

export type ReusableAgentEvent =
	/** A step was saved as a new reusable agent. */
	| 'saved'
	/** Edits to a linked agent were written back, propagating to every flow using it. */
	| 'updated'
	/** A saved agent was picked into a new step. */
	| 'linked'
	/** A linked step was forked back into a standalone agent. */
	| 'unlinked'
	/** A linked agent's unsaved draft was deployed alongside the flow that uses it. */
	| 'draft_deployed_with_flow'
	/** A linked agent's unsaved draft was left as a draft when its flow was deployed. */
	| 'draft_kept_on_deploy'

export function logReusableAgentUsage(event: ReusableAgentEvent): void {
	logFeatureUsage('ai_agent', 'reusable', { key: event })
}

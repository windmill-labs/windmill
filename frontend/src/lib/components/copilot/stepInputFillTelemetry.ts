import { logFeatureUsage } from '$lib/utils/featureUsage'

// Anonymous counters for the AI filling of a step's inputs. Same rules as every other
// `logFeatureUsage` caller: aggregated counts only, and the two keys below are the whole
// vocabulary — no argument name, expression or step id ever reaches here.

/** Which filler the user reached for: the per-field one, or the one above the whole form. */
export type StepInputFillScope = 'single' | 'all'

/** Counted where the user asks for a suggestion, not where one is accepted. */
export function logStepInputFill(scope: StepInputFillScope): void {
	logFeatureUsage('flow_step', 'ai_fill', { key: scope })
}

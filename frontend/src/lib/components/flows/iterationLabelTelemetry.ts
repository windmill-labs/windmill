import { logFeatureUsage } from '$lib/utils/featureUsage'

/**
 * `set`: a for-loop step gained an iteration label in the editor.
 * `searched`: an iteration was picked by typing label text into the iteration picker.
 * Counts only; the label expression and its values never reach here.
 */
export type IterationLabelAction = 'set' | 'searched'

export function logIterationLabel(action: IterationLabelAction): void {
	logFeatureUsage('flow_step', 'iteration_label', { key: action })
}

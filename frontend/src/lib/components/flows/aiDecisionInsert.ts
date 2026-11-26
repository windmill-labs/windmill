import type { FlowEditorContext } from './types'
import { addChoiceBranches } from './aiDecisionBranching'
import type { BranchKind } from './aiDecisionQuestions'
import { insertBranchOneAfter } from './branchOneInsert'

/** Insert a Branch to one right after the decision, with the question's branches (one per option
 *  of a choice, one `yes` for a yes/no), and select it. */
export function branchOnQuestion(
	ctx: FlowEditorContext,
	decisionId: string,
	question: string,
	kind: BranchKind,
	options: string[]
): Promise<void> {
	return insertBranchOneAfter(ctx, decisionId, (inserted) =>
		addChoiceBranches(inserted, decisionId, question, kind, options)
	)
}

/**
 * A chat tool call parked on a run form, shown on the item's deployed page instead of on the
 * card. The page's form edits this call's arguments and its Run answers the call; the tool
 * starts the job, so a page that also ran would start a second one for the same request.
 *
 * The call carries arguments and nothing else, which is why the page hides what it cannot
 * carry while it holds one: scheduling, tag override, PowerShell common parameters.
 */
export type PendingRun = {
	toolCallId: string
	/** The card's own draft, not a copy, so both forms edit one set of arguments. Gone once
	 * the call stops waiting, which a page mid-press outlives. */
	draftArgs: Record<string, any> | undefined
	setDraftArgs: (args: Record<string, any>) => void
	/** Plan mode promised no runs, and the dynamic-select helper and `password` fields write
	 * on mount and on keystroke — where nothing about confirming the run can gate them. */
	planModeActive: boolean
	/** Before the form mints a `password` into a workspace variable: false means this press
	 * writes nothing. Not in {@link submit}, which the form reaches only after minting. */
	claim: () => boolean
	/** For a claimed press that ends before {@link submit}, so the next is not refused. */
	release: () => void
	/** False when the call is no longer waiting — a stopped turn, or a job that failed to
	 * start — so the page can say so instead of leaving Run looking live. */
	submit: (args: Record<string, any>) => boolean
	decline: () => void
}

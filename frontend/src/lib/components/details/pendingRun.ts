/**
 * A chat tool call waiting for a run of this item, shown on the item's deployed page.
 *
 * The page runs it the way it runs anything — its own form, its own version, its own
 * arguments — and tells the call which job that was. The tool then follows that job instead
 * of starting one, so the chat reports on the run the reader actually started.
 */
export type PendingRun = {
	toolCallId: string
	/** The chat promised no runs and no writes. A form standing in for its card owes that
	 * too: a `password` mints as it is typed and the dynamic-select helper runs on mount,
	 * both before Run is ever pressed. */
	planModeActive: boolean
	/** What the chat proposed, for the form to open on. Read when the form is built, so this
	 * is the live draft of the card's own form rather than a copy to keep in step. */
	readonly proposedArgs: Record<string, any>
	/** The job the reader started here answers the call, with the arguments it was started
	 * with — the card has no other way to show what ran — and the schema it ran against, which
	 * a deploy can have moved on from the one the call fetched. Both are needed: redaction
	 * only visits the leaves a schema declares, so a file argument this page has and that one
	 * does not would reach the transcript as bytes. True when the call took the job: the chat
	 * then shows that run in place of this page, so the page must not navigate to it as well.
	 * False when the call settled while the job was starting, and the run is the reader's
	 * alone. */
	adopt: (jobId: string, args: Record<string, any>, schema: Record<string, any>) => boolean
	decline: () => void
	/** Claim the call's one job, before starting one. The card's form is live at the same
	 * time as this page's, and the call stays pending until `adopt` — which is a round trip
	 * later — so without this claim a Run on the card in between starts a second job, with
	 * its own arguments and no mention in the chat. False when the other surface claimed it
	 * first; send nothing and say so. */
	beginSubmit: () => boolean
	/** Hand the claim back when no job came of it, so the reader can try again. A started
	 * job needs no release: adopting it settles the call. */
	endSubmit: () => void
}

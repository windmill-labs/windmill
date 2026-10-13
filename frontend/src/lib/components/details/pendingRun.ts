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
	/** The job the reader started here answers the call. True when it did: the chat then shows
	 * that run in place of this page, so the page must not navigate to it as well. False when
	 * the call settled while the job was starting, and the run is the reader's alone. */
	adopt: (jobId: string) => boolean
	decline: () => void
}

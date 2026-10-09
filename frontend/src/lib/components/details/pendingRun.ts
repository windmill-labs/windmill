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
	/** The job the reader started here answers the call. */
	adopt: (jobId: string) => void
	decline: () => void
}

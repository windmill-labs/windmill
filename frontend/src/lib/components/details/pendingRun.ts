/**
 * A chat tool call parked on a deployed item's run form, when the reader chose to confirm it
 * on the item's own page instead of on the card. The page seeds its form from `args` and
 * routes Run to `submit` rather than starting a job of its own — the tool starts the job when
 * the call resumes, so a page that also ran would start a second one for the same request.
 *
 * The page is the card's form, relocated. That is why it cannot offer what the card cannot:
 * the call carries arguments and nothing else, so scheduling, tag override and
 * invisible-to-owner are hidden while it is adopted. They come back the moment it is not.
 */
export type PendingRun = {
	/** Which call is waiting. The page seeds its form once per call rather than once per
	 * mount: a tab already showing this item is reused for the next request, so a latch that
	 * only remembered "seeded" would leave the previous call's arguments on screen. */
	toolCallId: string
	/** What the form is waiting on — the model's proposal, then whatever has been typed into
	 * it since, on either surface. The card's own draft, not a copy: a form bound to it edits
	 * the one set of arguments this call will run with, so there is nothing to carry back when
	 * the page goes away. Gone once the call stops waiting, which a page mid-press outlives. */
	draftArgs: Record<string, any> | undefined
	/** Replace them wholesale. Binding writes through here only when something hands the form
	 * a new object; ordinary editing mutates {@link draftArgs} in place. */
	setDraftArgs: (args: Record<string, any>) => void
	/** Whether the chat is in plan mode, which promised no runs. The page is standing in for
	 * a chat card, so it owes that promise too — and the dynamic-select helper runs its job on
	 * mount, where nothing about confirming the run can gate it. */
	planModeActive: boolean
	/** Take the call, before the form mints a `password` field into an ephemeral workspace
	 * variable. False means this press runs nothing: plan mode is on, or a press is already
	 * in flight. Both have to be answered here rather than in {@link submit}, which the form
	 * only reaches after it has written those variables. */
	claim: () => boolean
	/** Give the call back when a claimed press ends without reaching {@link submit}, so the
	 * next one is not refused as a double press. */
	release: () => void
	/** Hand the form's current arguments to the waiting call. False when it is no longer
	 * waiting — a stopped turn, or a job that failed to start — so the page can say so
	 * instead of leaving Run looking live. */
	submit: (args: Record<string, any>) => boolean
	/** Refuse the call, as the card's Cancel does. Not an undo of the prefill: there is
	 * nothing to restore it to, and leaving the call parked with an emptied form would strand
	 * the turn. */
	decline: () => void
}

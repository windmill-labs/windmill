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
	/** What the model proposed, already narrowed to the deployed schema by the tool. */
	args: Record<string, any>
	/** Hand the form's current arguments to the waiting call. False when it is no longer
	 * waiting — a stopped turn, or a job that failed to start — so the page can say so
	 * instead of leaving Run looking live. */
	submit: (args: Record<string, any>) => boolean
	/** Refuse the call, as the card's Cancel does. Not an undo of the prefill: there is
	 * nothing to restore it to, and leaving the call parked with an emptied form would strand
	 * the turn. `reason` is what the model is told instead of "cancelled" — for the page that
	 * settles the call by running the item some other way, where reporting a refusal would
	 * have the model say nothing ran. */
	decline: (reason?: string) => void
}

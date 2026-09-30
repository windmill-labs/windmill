import type { Schema } from '$lib/common'

/**
 * A chat tool call parked on a deployed item's run form: the model proposed these
 * arguments and is waiting for the reader to run them, or to decline.
 *
 * The page seeds its form from `args` and routes Run to `submit` rather than starting a
 * job of its own — the tool starts the job when the call resumes, so a page that also ran
 * would start a second one for the same request.
 */
export type PendingRun = {
	/** Which call is waiting. The page seeds its form once per call rather than once per
	 * mount: a tab already showing this item is reused for the next request, so a latch that
	 * only remembers "seeded" would leave the previous call's arguments on screen. */
	toolCallId: string
	/** What the model proposed, before the page narrows it to its own schema. */
	args: Record<string, any>
	/** Hand the form's current arguments to the waiting call. */
	submit: (args: Record<string, any>) => void
	/** Refuse the call, as the chat card's Cancel does. Not an undo of the prefill: there is
	 * nothing to restore it to, and leaving the call parked with an emptied form would strand
	 * the turn. */
	decline: () => void
}

/**
 * The proposed arguments, less the ones this schema has no field for. The model writes
 * them against the item it has been editing, whose parameters can differ from what is
 * deployed — dropping the strays leaves a form the reader can still fill and run, where
 * passing them whole would send arguments the deployed version never declared.
 */
export function argsForSchema(
	args: Record<string, any> | undefined,
	schema: Schema | undefined
): Record<string, any> {
	if (!args) return {}
	const declared = schema?.properties
	// No schema yet (still loading, or a runnable that takes none) means nothing to narrow
	// against; seeding wholesale here would be a guess, so seed nothing.
	if (!declared) return {}
	return Object.fromEntries(Object.entries(args).filter(([k]) => k in declared))
}

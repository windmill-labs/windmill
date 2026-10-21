/**
 * The composer's box, shared by both of AIChatInput's branches — the rich
 * ContextTextarea and the plain textarea a host without @-context gets.
 *
 * Border and rounding live on the WRAPPER, never on the field, so the chip rows
 * (context, files, images) sit inside the box above the text. The field's own
 * @tailwindcss/forms border, ring and background are neutralised so only the
 * wrapper reads as the input.
 *
 * The disabled treatment is on the wrapper for the same reason: `disabled` on the
 * field alone leaves it looking exactly like a usable one, so the only cue that
 * typing is refused is placeholder text the eye reads as an invitation.
 */

const BOX_BASE = 'w-full scroll-pb-2 rounded-md border border-border-light transition-colors'

export function composerBoxClass(disabled: boolean = false): string {
	return `${BOX_BASE} ${
		disabled
			? 'bg-surface-disabled cursor-not-allowed'
			: 'bg-surface-input focus-within:border-border-selected'
	}`
}

/** Applied to the field inside the box; without it the field draws a second border. */
export const COMPOSER_FIELD_RESET =
	'!border-transparent !bg-transparent !shadow-none focus:!border-transparent focus:!ring-0 disabled:cursor-not-allowed disabled:placeholder:text-disabled'

/** A caret or selected range in the composer's text. */
export type ComposerSelection = { start: number; end: number }

const GLOBAL_PLACEHOLDER_SUGGESTIONS = [
	'Write a hello-world flow',
	'Create a script that lists files in an S3 bucket',
	'Build a CRUD app for a customer table',
	'Schedule a daily cleanup of old runs',
	'Wrap an existing script into a flow with retries',
	'Add an HTTP trigger to an existing script',
	'Generate a report from a SQL query and email it',
	'Create a Postgres resource and a script that queries it',
	'Refactor a script to add error handling',
	'List my workspace flows and scripts'
]
// An operator authors nothing, so nothing above is theirs to ask for; one with the
// workspace's builder right can compose flows out of deployed scripts as well.
const OPERATOR_PLACEHOLDER_SUGGESTIONS = [
	'List my workspace flows and scripts',
	'Show the runs that failed today and why',
	'Find the script that sends a report and run it',
	'Rerun the last failed run of a flow'
]
const BUILDER_PLACEHOLDER_SUGGESTIONS = [
	...OPERATOR_PLACEHOLDER_SUGGESTIONS,
	'Compose a flow that chains two deployed scripts'
]

/**
 * The global-mode composer's placeholder: one suggestion the user may ask for. `pick` is a
 * number in [0, 1) the caller draws once, so the text does not shuffle while it is read.
 */
export function globalComposerSuggestion(
	pick: number,
	canOpenEditor: (kind: string) => boolean
): string {
	const pool = canOpenEditor('script')
		? GLOBAL_PLACEHOLDER_SUGGESTIONS
		: canOpenEditor('flow')
			? BUILDER_PLACEHOLDER_SUGGESTIONS
			: OPERATOR_PLACEHOLDER_SUGGESTIONS
	return pool[Math.floor(pick * pool.length)]
}

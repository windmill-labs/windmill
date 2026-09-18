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

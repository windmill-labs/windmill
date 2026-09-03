/**
 * The composer's box, shared by both of AIChatInput's branches — the rich
 * ContextTextarea and the plain textarea a host without @-context gets.
 *
 * Border and rounding live on the WRAPPER, never on the field, so the chip rows
 * (context, files, images) sit inside the box above the text. The field's own
 * @tailwindcss/forms border, ring and background are neutralised so only the
 * wrapper reads as the input.
 */

export const COMPOSER_BOX =
	'w-full scroll-pb-2 bg-surface-input rounded-md border border-border-light focus-within:border-border-selected transition-colors'

/** Applied to the field inside COMPOSER_BOX; without it the field draws a second border. */
export const COMPOSER_FIELD_RESET =
	'!border-transparent !bg-transparent !shadow-none focus:!border-transparent focus:!ring-0'

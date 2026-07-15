/**
 * Shared prose (Tailwind typography) stacks for markdown renders, one source of
 * truth so surfaces don't each roll their own and drift apart. Compose at the
 * call site with layout-only classes (padding, width, bg); anything typographic
 * belongs here.
 *
 * - 'xs': micro scale for dense secondary panes (chat reasoning blocks)
 * - 'sm': compact chat-bubble scale (assistant messages, flow/app chat, settings)
 * - 'doc': same rhythm and body size as 'sm', with a taller heading ramp
 *   (lg/base/sm) and semibold headings for document-like surfaces (artifacts)
 */

// Kept as literal template parts: Tailwind's scanner reads class names verbatim
// from this file, so every token must appear as plain text.
// content-none strips the typography plugin's decorative backticks around
// inline code (code::before/::after), which read as literal ` characters.
const base =
	'prose dark:prose-invert max-w-full break-words prose-a:break-words prose-code:break-words prose-code:before:content-none prose-code:after:content-none prose-code:bg-surface-secondary/50 prose-code:rounded prose-code:px-1 prose-code:py-0.5 prose-code:font-normal prose-table:block prose-table:max-w-full prose-table:overflow-x-auto'

// One vertical rhythm for sm/doc; heading margins stay per-preset (fixed, not
// the plugin's em-based ones) so 'doc' can breathe more between sections.
const rhythm = 'prose-sm leading-snug prose-ul:!pl-6'

const bodyXs =
	'text-primary prose-p:text-primary prose-li:text-primary prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs prose-table:text-xs'

export const markdownProse = {
	xs: `${base} prose-sm leading-snug prose-ul:!pl-5 prose-p:text-2xs prose-li:text-2xs prose-code:text-2xs prose-pre:text-2xs prose-headings:font-medium prose-headings:text-secondary prose-headings:mt-2 prose-headings:mb-1 prose-h1:text-2xs prose-h2:text-2xs prose-h3:text-2xs prose-h4:text-2xs prose-h5:text-2xs prose-h6:text-2xs prose-strong:text-secondary`,
	sm: `${base} ${rhythm} ${bodyXs} prose-headings:mt-3 prose-headings:mb-1 prose-headings:font-medium prose-headings:text-emphasis prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs`,
	doc: `${base} ${rhythm} ${bodyXs} prose-headings:mt-8 prose-headings:mb-2 prose-headings:first:mt-0 prose-headings:font-semibold prose-headings:text-emphasis prose-h1:text-lg prose-h2:text-base prose-h3:text-sm prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs prose-pre:bg-transparent prose-pre:p-0`
} as const

export type MarkdownProseSize = keyof typeof markdownProse

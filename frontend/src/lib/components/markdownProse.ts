/**
 * Shared prose (Tailwind typography) stacks for markdown renders, one source of
 * truth so surfaces don't each roll their own and drift apart. Compose at the
 * call site with layout-only classes (padding, width, bg); anything typographic
 * belongs here.
 *
 * - 'xs': micro scale for dense secondary panes (chat reasoning blocks, group notes)
 * - 'sm': compact chat-bubble scale (assistant messages, flow/app chat, settings)
 *
 * Artifacts are not styled here: they use `artifacts/artifactDocument.css`, which their
 * HTML export also inlines.
 *
 * h1 and h2 step above the body size in every preset, so `#` and `##` read as
 * headings rather than bold body text. Below that the tight 'xs' and 'sm' scales
 * run out of room, and h3 down differentiates by weight and colour alone.
 */

// Kept as literal template parts: Tailwind's scanner reads class names verbatim
// from this file, so every token must appear as plain text.
// content-none strips the typography plugin's decorative backticks around
// inline code (code::before/::after), which read as literal ` characters.
// [&>:first-child]:mt-0 re-applies the plugin's first-block reset, which our
// explicit prose-headings:mt-* would otherwise override (note: the composed
// variant prose-headings:first: attaches :first-child to the wrapper — wrong).
const base =
	'prose dark:prose-invert max-w-full break-words [&>:first-child]:mt-0 prose-a:break-words prose-code:break-words prose-code:before:content-none prose-code:after:content-none prose-code:bg-surface-secondary/50 prose-code:rounded prose-code:px-1 prose-code:py-0.5 prose-code:font-normal prose-table:block prose-table:max-w-full prose-table:overflow-x-auto'

// Heading margins stay fixed rather than the plugin's em-based ones.
const rhythm = 'prose-sm leading-snug'

const bodyXs =
	'text-primary prose-p:text-primary prose-li:text-primary prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs prose-table:text-xs'

export const markdownProse = {
	xs: `${base} prose-sm leading-snug prose-p:text-2xs prose-li:text-2xs prose-code:text-2xs prose-pre:text-2xs prose-headings:font-medium prose-headings:text-secondary prose-headings:mt-2 prose-headings:mb-1 prose-h1:text-sm prose-h2:text-xs prose-h3:text-2xs prose-h4:text-2xs prose-h5:text-2xs prose-h6:text-2xs prose-strong:text-secondary`,
	sm: `${base} ${rhythm} ${bodyXs} prose-headings:mt-3 prose-headings:mb-1 prose-headings:font-medium prose-headings:text-emphasis prose-h1:text-base prose-h2:text-sm prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs`
} as const

export type MarkdownProseSize = keyof typeof markdownProse

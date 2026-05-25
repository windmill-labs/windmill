// Shared styles for the chat input footer bar pills (@ context, autonomy
// mode, AI mode, model selector). Keeping them in one place ensures the
// controls share the same height/padding/typography so the bar stays aligned.
const PILL_BASE =
	'flex flex-row items-center gap-0.5 h-6 px-1.5 rounded-md border text-xs text-primary font-normal min-w-0'

// Interactive pill (opens a dropdown/popover on click).
export const CHAT_BAR_PILL = `${PILL_BASE} bg-surface hover:bg-surface-hover transition-colors`

// Non-interactive pill (single option, nothing to pick).
export const CHAT_BAR_PILL_STATIC = `${PILL_BASE} bg-surface`

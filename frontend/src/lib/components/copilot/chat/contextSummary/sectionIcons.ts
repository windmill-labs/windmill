import { BookOpen, Boxes, Plug, ScrollText } from 'lucide-svelte'
import type { ContextSectionId } from './contextSummary'

/** The settings modal's section icons, so both surfaces read as the same thing. */
export const CONTEXT_SECTION_ICONS = {
	tools: Boxes,
	skills: BookOpen,
	instructions: ScrollText,
	mcp: Plug
} satisfies Record<ContextSectionId, unknown>

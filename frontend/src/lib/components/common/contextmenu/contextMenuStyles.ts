/**
 * Shared styles for context menu components
 * Ensures visual consistency across all context menu implementations
 */

/**
 * Base container styles for context menu
 * @param zIndex - Optional z-index override (default: 'z-50')
 */
export function getContextMenuContainerClass(zIndex: string = 'z-50'): string {
	return `${zIndex} flex flex-col gap-1 min-w-[12rem] overflow-hidden rounded-md border bg-surface p-1 shadow-md`
}

/**
 * Base styles for context menu items
 */
export const CONTEXT_MENU_ITEM_BASE_CLASS =
	'relative flex cursor-default select-none items-center rounded-md px-2 py-1.5 text-xs outline-none transition-colors'

/**
 * Hover state styles for context menu items (standard CSS hover)
 */
export const CONTEXT_MENU_ITEM_HOVER_CLASS = 'hover:bg-surface-hover'

/**
 * Hover state styles for context menu items (Melt UI data attribute)
 */
export const CONTEXT_MENU_ITEM_HOVER_MELT_CLASS = 'data-[highlighted]:bg-surface-hover'

/**
 * Disabled state styles for context menu items
 */
export const CONTEXT_MENU_ITEM_DISABLED_CLASS = 'pointer-events-none opacity-50'

/**
 * Divider styles for context menu
 */
export const CONTEXT_MENU_DIVIDER_CLASS = 'my-1 h-px bg-border-light'

/**
 * Melt UI animation classes for context menu
 */
export const CONTEXT_MENU_ANIMATION_CLASSES =
	'data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2'


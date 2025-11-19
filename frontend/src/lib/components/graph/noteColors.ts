// Note color definitions with Tailwind classes for light and dark mode
export enum NoteColor {
	YELLOW = 'yellow',
	BLUE = 'blue',
	GREEN = 'green',
	PURPLE = 'purple',
	PINK = 'pink',
	ORANGE = 'orange',
	RED = 'red',
	CYAN = 'cyan',
	LIME = 'lime',
	GRAY = 'gray'
}

export interface NoteColorConfig {
	background: string
	outline: string
	outlineHover: string
	text: string
	hover: string
}

// Color configurations for each note color with dark mode support
export const NOTE_COLORS: Record<NoteColor, NoteColorConfig> = {
	[NoteColor.YELLOW]: {
		background: 'bg-yellow-200 dark:bg-yellow-900',
		outline: 'outline-yellow-300 dark:outline-yellow-600',
		outlineHover: 'outline-yellow-300/60 dark:outline-yellow-600/60',
		text: 'text-yellow-900 dark:text-yellow-100',
		hover: 'hover:bg-yellow-200 dark:hover:bg-yellow-800'
	},
	[NoteColor.BLUE]: {
		background: 'bg-blue-100 dark:bg-blue-950',
		outline: 'outline-blue-300 dark:outline-blue-600',
		outlineHover: 'outline-blue-300/60 dark:outline-blue-600/60',
		text: 'text-blue-900 dark:text-blue-100',
		hover: 'hover:bg-blue-200 dark:hover:bg-blue-800'
	},
	[NoteColor.GREEN]: {
		background: 'bg-green-200 dark:bg-green-900',
		outline: 'outline-green-300 dark:outline-green-600',
		outlineHover: 'outline-green-300/60 dark:outline-green-600/60',
		text: 'text-green-900 dark:text-green-100',
		hover: 'hover:bg-green-200 dark:hover:bg-green-800'
	},
	[NoteColor.PURPLE]: {
		background: 'bg-purple-200 dark:bg-purple-900',
		outline: 'outline-purple-300 dark:outline-purple-600',
		outlineHover: 'outline-purple-300/60 dark:outline-purple-600/60',
		text: 'text-purple-900 dark:text-purple-100',
		hover: 'hover:bg-purple-200 dark:hover:bg-purple-800'
	},
	[NoteColor.PINK]: {
		background: 'bg-pink-200 dark:bg-pink-900',
		outline: 'outline-pink-300 dark:outline-pink-600',
		outlineHover: 'outline-pink-300/60 dark:outline-pink-600/60',
		text: 'text-pink-900 dark:text-pink-100',
		hover: 'hover:bg-pink-200 dark:hover:bg-pink-800'
	},
	[NoteColor.ORANGE]: {
		background: 'bg-orange-200 dark:bg-orange-900',
		outline: 'outline-orange-300 dark:outline-orange-600',
		outlineHover: 'outline-orange-300/60 dark:outline-orange-600/60',
		text: 'text-orange-900 dark:text-orange-100',
		hover: 'hover:bg-orange-200 dark:hover:bg-orange-800'
	},
	[NoteColor.RED]: {
		background: 'bg-red-200 dark:bg-red-900',
		outline: 'outline-red-300 dark:outline-red-600',
		outlineHover: 'outline-red-300/60 dark:outline-red-600/60',
		text: 'text-red-900 dark:text-red-100',
		hover: 'hover:bg-red-200 dark:hover:bg-red-800'
	},
	[NoteColor.CYAN]: {
		background: 'bg-cyan-200 dark:bg-cyan-900',
		outline: 'outline-cyan-300 dark:outline-cyan-600',
		outlineHover: 'outline-cyan-300/60 dark:outline-cyan-600/60',
		text: 'text-cyan-900 dark:text-cyan-100',
		hover: 'hover:bg-cyan-200 dark:hover:bg-cyan-800'
	},
	[NoteColor.LIME]: {
		background: 'bg-lime-200 dark:bg-lime-900',
		outline: 'outline-lime-300 dark:outline-lime-600',
		outlineHover: 'outline-lime-300/60 dark:outline-lime-600/60',
		text: 'text-lime-900 dark:text-lime-100',
		hover: 'hover:bg-lime-200 dark:hover:bg-lime-800'
	},
	[NoteColor.GRAY]: {
		background: 'bg-gray-200 dark:bg-gray-800',
		outline: 'outline-gray-300 dark:outline-gray-600',
		outlineHover: 'outline-gray-300/60 dark:outline-gray-600/60',
		text: 'text-gray-900 dark:text-gray-100',
		hover: 'hover:bg-gray-200 dark:hover:bg-gray-700'
	}
}

// Color swatch colors for the picker (solid colors for the palette dots)
export const NOTE_COLOR_SWATCHES: Record<NoteColor, string> = {
	[NoteColor.YELLOW]: 'bg-yellow-400',
	[NoteColor.BLUE]: 'bg-blue-400',
	[NoteColor.GREEN]: 'bg-green-400',
	[NoteColor.PURPLE]: 'bg-purple-400',
	[NoteColor.PINK]: 'bg-pink-400',
	[NoteColor.ORANGE]: 'bg-orange-400',
	[NoteColor.RED]: 'bg-red-400',
	[NoteColor.CYAN]: 'bg-cyan-400',
	[NoteColor.LIME]: 'bg-lime-400',
	[NoteColor.GRAY]: 'bg-gray-400'
}

// Default note color
export const DEFAULT_NOTE_COLOR = NoteColor.GREEN
export const DEFAULT_GROUP_NOTE_COLOR = NoteColor.BLUE

/**
 * Get the next available color that's not in the used colors set
 * Cycles through all available colors in order
 */
export function getNextAvailableColor(usedColors: Set<NoteColor>): NoteColor {
	const allColors = Object.values(NoteColor)

	// Find first unused color
	for (const color of allColors) {
		if (!usedColors.has(color)) {
			return color
		}
	}

	// If all colors are used, return the default
	return DEFAULT_GROUP_NOTE_COLOR
}

// Minimum note size constraints
export const MIN_NOTE_WIDTH = 275
export const MIN_NOTE_HEIGHT = 60

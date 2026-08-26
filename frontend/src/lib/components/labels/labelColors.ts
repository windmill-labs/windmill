import type { BadgeColor } from '$lib/components/common/badge/model'
import type { LabelColor } from '$lib/gen'

/** The pickable colors, in the order the swatch grid lays them out. */
export const LABEL_COLORS: readonly LabelColor[] = [
	'yellow',
	'blue',
	'green',
	'purple',
	'pink',
	'orange',
	'red',
	'cyan',
	'lime',
	'gray'
] as const

/** Solid dots for the picker. */
export const LABEL_COLOR_SWATCHES: Record<LabelColor, string> = {
	yellow: 'bg-yellow-400',
	blue: 'bg-blue-400',
	green: 'bg-green-400',
	purple: 'bg-purple-400',
	pink: 'bg-pink-400',
	orange: 'bg-orange-400',
	red: 'bg-red-400',
	cyan: 'bg-cyan-400',
	lime: 'bg-lime-400',
	gray: 'bg-gray-400'
}

/**
 * What an uncolored label looks like — the blue chip labels have always had, so
 * a workspace that never picks a color sees no change. Deliberately not gray:
 * gray is a color someone can choose, and the folder icon, not the color, is
 * what marks a label as inherited.
 */
export const DEFAULT_LABEL_BADGE_COLOR: BadgeColor = 'blue'

export function labelBadgeColor(color: LabelColor | undefined): BadgeColor {
	return color ?? DEFAULT_LABEL_BADGE_COLOR
}

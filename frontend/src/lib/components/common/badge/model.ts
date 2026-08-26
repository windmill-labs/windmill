type BaseColor = 'blue' | 'gray' | 'red' | 'green' | 'yellow' | 'indigo' | 'orange' | 'violet'
// Reachable only in their light form: they exist to round the label palette out to ten
// distinguishable hues, and nothing asks a label chip for the filled `dark-` treatment.
type LightOnlyColor = 'purple' | 'pink' | 'cyan' | 'lime'
export const ColorModifier = 'dark-'
export type BadgeColor =
	| BaseColor
	| LightOnlyColor
	| 'transparent'
	| `${typeof ColorModifier}${BaseColor}`

export interface BadgeIconProps {
	position?: 'left' | 'right'
	icon: any
}

export const badgeColors: Record<BadgeColor, string> = {
	gray: 'bg-surface-sunken text-primary',
	blue: 'bg-blue-50 text-blue-800 dark:text-blue-100 dark:bg-blue-700/40',
	red: 'bg-red-100 text-red-800 dark:bg-red-700/40 dark:text-red-100',
	green: 'bg-green-100 text-green-700 dark:bg-green-700/40 dark:text-green-100',
	yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-700/40 dark:text-yellow-100',
	orange: 'bg-orange-100 text-orange-800 dark:bg-orange-700/40 dark:text-orange-100',
	indigo: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-700/40 dark:text-indigo-100',
	violet: 'bg-violet-100 text-violet-800 dark:bg-violet-800/30 dark:text-violet-100',
	purple: 'bg-purple-100 text-purple-800 dark:bg-purple-700/40 dark:text-purple-100',
	pink: 'bg-pink-100 text-pink-800 dark:bg-pink-700/40 dark:text-pink-100',
	cyan: 'bg-cyan-100 text-cyan-800 dark:bg-cyan-700/40 dark:text-cyan-100',
	lime: 'bg-lime-100 text-lime-800 dark:bg-lime-700/40 dark:text-lime-100',
	['dark-gray']: 'bg-gray-500 text-gray-100 dark:bg-gray-600 dark:text-gray-200',
	['dark-blue']: 'bg-blue-500 text-blue-100 dark:bg-blue-600 dark:text-blue-200',
	['dark-red']: 'bg-red-500 text-white dark:bg-red-600 dark:text-red-100',
	['dark-green']: 'bg-green-500 text-green-100 dark:bg-green-600 dark:text-green-200',
	['dark-yellow']: 'bg-yellow-500 text-white dark:bg-yellow-600 dark:text-yellow-100',
	['dark-indigo']: 'bg-indigo-500 text-indigo-100 dark:bg-indigo-600 dark:text-indigo-200',
	['dark-orange']: 'bg-orange-500 text-orange-100 dark:bg-orange-600 dark:text-orange-200',
	['dark-violet']: 'bg-violet-500 text-violet-100 dark:bg-violet-600 dark:text-violet-200',
	transparent: 'bg-transparent border'
}

export const badgeSelectedColors: Record<BadgeColor, string> = {
	gray: 'bg-surface-input text-primary',
	blue: 'bg-blue-500 text-white dark:bg-blue-600',
	red: 'bg-red-500 text-white dark:bg-red-600',
	green: 'bg-green-500 text-white dark:bg-green-600',
	yellow: 'bg-yellow-500 text-white dark:bg-yellow-600',
	orange: 'bg-orange-500 text-white dark:bg-orange-600',
	indigo: 'bg-indigo-500 text-white dark:bg-indigo-600',
	violet: 'bg-violet-500 text-white dark:bg-violet-600',
	purple: 'bg-purple-500 text-white dark:bg-purple-600',
	pink: 'bg-pink-500 text-white dark:bg-pink-600',
	cyan: 'bg-cyan-500 text-white dark:bg-cyan-600',
	lime: 'bg-lime-500 text-white dark:bg-lime-600',
	['dark-gray']: 'bg-gray-700 text-gray-100 dark:bg-gray-800 dark:text-gray-200',
	['dark-blue']: 'bg-blue-700 text-blue-100 dark:bg-blue-800 dark:text-blue-200',
	['dark-red']: 'bg-red-700 text-white dark:bg-red-800 dark:text-red-100',
	['dark-green']: 'bg-green-700 text-green-100 dark:bg-green-800 dark:text-green-200',
	['dark-yellow']: 'bg-yellow-600 text-white dark:bg-yellow-700 dark:text-yellow-100',
	['dark-indigo']: 'bg-indigo-700 text-indigo-100 dark:bg-indigo-800 dark:text-indigo-200',
	['dark-orange']: 'bg-orange-700 text-orange-100 dark:bg-orange-800 dark:text-orange-200',
	['dark-violet']: 'bg-violet-700 text-violet-100 dark:bg-violet-800 dark:text-violet-200',
	transparent: 'bg-surface-accent-selected text-accent border-gray-400 dark:border-gray-500'
}

export const badgeHovers: Partial<Record<BadgeColor, string>> = {
	gray: 'hover:bg-surface-hover',
	blue: 'hover:bg-blue-100 dark:hover:bg-blue-700/60',
	red: 'hover:bg-red-200 dark:hover:bg-red-500/25',
	green: 'hover:bg-green-200 dark:hover:bg-green-500/25',
	yellow: 'hover:bg-yellow-200 dark:hover:bg-yellow-500/25',
	indigo: 'hover:bg-indigo-200 dark:hover:bg-indigo-500/25',
	orange: 'hover:bg-orange-200 dark:hover:bg-orange-500/25',
	violet: 'hover:bg-violet-200 dark:hover:bg-violet-500/25',
	purple: 'hover:bg-purple-200 dark:hover:bg-purple-500/25',
	pink: 'hover:bg-pink-200 dark:hover:bg-pink-500/25',
	cyan: 'hover:bg-cyan-200 dark:hover:bg-cyan-500/25',
	lime: 'hover:bg-lime-200 dark:hover:bg-lime-500/25',
	['dark-gray']: 'hover:bg-gray-600 dark:hover:bg-gray-700',
	['dark-blue']: 'hover:bg-blue-600 dark:hover:bg-blue-700',
	['dark-red']: 'hover:bg-red-600 dark:hover:bg-red-700',
	['dark-green']: 'hover:bg-green-600 dark:hover:bg-green-700',
	['dark-yellow']: 'hover:bg-yellow-600 dark:hover:bg-yellow-700',
	['dark-indigo']: 'hover:bg-indigo-600 dark:hover:bg-indigo-700',
	['dark-orange']: 'hover:bg-orange-600 dark:hover:bg-orange-700',
	['dark-violet']: 'hover:bg-violet-600 dark:hover:bg-violet-700',
	transparent: 'hover:bg-surface-hover dark:hover:border-gray-500'
}

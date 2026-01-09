export type AlertType = 'success' | 'error' | 'warning' | 'info'

export const classes: Record<AlertType, Record<string, string>> = {
	info: {
		bgClass: 'bg-blue-50 border-blue-100 border dark:bg-blue-900/60 dark:border-blue-700/40',
		iconClass: 'text-blue-600 dark:text-blue-50',
		titleClass: 'text-blue-700 dark:text-blue-50',
		descriptionClass: 'text-blue-600 dark:text-blue-50/80'
	},
	warning: {
		bgClass:
			'bg-yellow-50 border-yellow-200 border dark:bg-yellow-900/40 dark:border-yellow-700/40',
		iconClass: 'text-yellow-500 dark:text-yellow-300/90',
		titleClass: 'text-yellow-800 dark:text-yellow-100/90',
		descriptionClass: 'text-yellow-700 dark:text-yellow-100/90'
	},
	error: {
		bgClass: 'bg-red-50 border-red-200 border dark:bg-red-900/40 dark:border-red-700/40',
		iconClass: 'text-red-500 dark:text-red-300/90',
		titleClass: 'text-red-800 dark:text-red-100/90',
		descriptionClass: 'text-red-700 dark:text-red-100/90'
	},
	success: {
		bgClass: 'bg-green-50 border-green-200 border dark:bg-green-900/40 dark:border-green-700/40',
		iconClass: 'text-green-500 dark:text-green-300/90',
		titleClass: 'text-green-800 dark:text-green-100/90',
		descriptionClass: 'text-green-700 dark:text-green-100/90'
	}
}

export function AIBtnClasses(state: 'default' | 'selected' | 'green' | 'accent' = 'default') {
	return twMerge(
		['selected', 'default'].includes(state) ? 'text-ai !border-ai/20 hover:bg-ai/15' : '',
		{
			default: '',
			selected: 'bg-ai/10',
			green:
				'bg-green-50 hover:bg-green-50 dark:bg-green-400/15 dark:hover:bg-green-400/15 text-green-800 border-green-200 dark:border-green-300/60 dark:text-green-400',
			accent:
				'dark:bg-magenta-900 bg-magenta-600 text-white hover:bg-magenta-700 dark:hover:bg-magenta-950'
		}[state]
	)
}

import { twMerge } from 'tailwind-merge'

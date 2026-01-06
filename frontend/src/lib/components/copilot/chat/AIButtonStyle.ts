export function AIBtnClasses(state: 'default' | 'selected' | 'green' = 'default') {
	return twMerge(
		['selected', 'default'].includes(state) ? 'text-ai !border-ai/20 hover:bg-ai/15' : '',
		{
			default: '',
			selected: 'bg-ai/10',
			green:
				'bg-green-50 hover:bg-green-50 dark:bg-green-400/15 dark:hover:bg-green-400/15 text-green-800 border-green-200 dark:border-green-300/60 dark:text-green-400'
		}[state]
	)
}

import { twMerge } from 'tailwind-merge'

export const BUTTON_COLORS = ['blue', 'red', 'dark', 'light', 'green', 'gray', 'none'] as const

export namespace ButtonType {
	export type Size = 'xs3' | 'xs2' | 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	export type Color = string
	export type Variant = 'contained' | 'border' | 'divider'
	export type Target = '_self' | '_blank'
	export type Element = HTMLButtonElement | HTMLAnchorElement
	export interface Icon {
		icon?: any | undefined
		classes?: string
		faIcon?: any | undefined
		props?: any
	}

	export const FontSizeClasses: Record<ButtonType.Size, string> = {
		xs3: 'text-2xs',
		xs2: 'text-xs',
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	} as const

	export const SpacingClasses: Record<ButtonType.Size, Record<ButtonType.Variant, string>> = {
		xs3: {
			border: 'px-0.5 py-[1px] !rounded-xs',
			contained: 'px-0.5 py-[1px]',
			divider: ''
		},
		xs2: {
			border: 'px-2 py-[4px]',
			contained: 'px-2 py-[4px]',
			divider: ''
		},
		xs: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]',
			divider: ''
		},
		sm: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]',
			divider: ''
		},
		md: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]',
			divider: ''
		},
		lg: {
			border: 'px-4 py-[8px]',
			contained: 'px-4 py-[9px]',
			divider: ''
		},
		xl: {
			border: 'px-4 py-[8px]',
			contained: 'px-4 py-[9px]',
			divider: ''
		}
	} as const

	export interface ItemProps {
		size: Size
		color: Color
	}

	export const ColorVariants: Record<ButtonType.Color, Record<ButtonType.Variant, string>> = {
		none: {
			border: '',
			contained: '',
			divider: ''
		},

		blue: {
			border:
				'border-frost-500 dark:border-frost-300 hover:border-frost-700 dark:hover:border-frost-400 focus-visible:border-frost-700 bg-surface hover:bg-frost-100 dark:hover:bg-frost-900 focus-visible:bg-frost-100 focus-visible:dark:text-frost-100 dark:focus-visible:bg-frost-900 text-frost-500 dark:text-frost-300 dark:hover:text-frost-400 hover:text-frost-700 focus-visible:text-frost-700 focus-visible:ring-frost-300',
			contained:
				'bg-frost-500 hover:bg-frost-700 focus-visible:bg-frost-700 text-white focus-visible:ring-frost-300 dark:bg-frost-500/90 dark:hover:bg-frost-600/90',
			divider: 'divide-x divide-frost-600'
		},
		marine: {
			border:
				'border-marine-300 dark:border-marine-200 hover:border-marine-500 dark:hover:border-marine-400 focus-visible:border-marine-500 bg-surface hover:bg-marine-500 dark:hover:bg-marine-400 focus-visible:bg-marine-100 focus-visible:dark:text-marine-50 dark:focus-visible:bg-marine-500 text-marine-50 dark:text-marine-50 dark:hover:text-marine-50 hover:text-marine-50 focus-visible:text-marine-300 focus-visible:ring-marine-200',
			contained:
				'bg-marine-300 hover:bg-marine-500 focus-visible:bg-marine-500 text-gray-50 focus-visible:ring-marine-200 dark:bg-marine-200 dark:hover:bg-marine-400',
			divider: 'divide-x divide-marine-500'
		},
		red: {
			border:
				'border-red-600/60 hover:border-red-600 bg-surface hover:bg-red-100 text-red-600 hover:text-red-700 focus-visible:ring-red-300 dark:border-red-400/90 dark:text-red-400 dark:hover:border-red-400 dark:hover:bg-red-500/60 dark:hover:text-red-100',
			contained:
				'bg-red-600 hover:bg-red-800 text-white focus-visible:ring-red-300  dark:border-red-400/70 dark:bg-red-700/90 dark:hover:bg-red-900 dark:hover:border-red-300 dark:text-primary',
			divider: 'divide-x divide-red-700'
		},
		green: {
			border:
				'border-green-600 hover:border-green-700 bg-surface hover:bg-green-100 text-green-600 hover:text-green-700 focus-visible:ring-green-300 dark:hover:bg-green-800 dark:border-green-400/90',
			contained:
				'bg-green-600 hover:bg-green-700 text-white focus-visible:ring-green-300 dark:bg-green-700/90 dark:hover:bg-green-800',
			divider: 'divide-x divide-green-700'
		},
		dark: {
			border:
				'border-marine-300 bg-surface hover:bg-surface-hover focus-visible:bg-surface-hover text-primary hover:text-secondary focus-visible:text-secondary focus-visible:ring-surface-selected-inverse dark:border-marine-200',
			contained:
				'bg-marine-400 hover:bg-marine-200 focus-visible:bg-surface-hover-inverse text-primary-inverse focus-visible:ring-surface-selected-inverse dark:bg-marine-50 dark:hover:bg-marine-50/70 dark:text-primary-inverse',
			divider: 'divide-x divide-gray-800 dark:divide-gray-200'
		},
		gray: {
			border:
				'border-gray-500 dark:border-gray-300 hover:border-gray-700 dark:hover:border-gray-400 focus-visible:border-gray-700 bg-surface hover:bg-gray-100 dark:hover:bg-gray-900 focus-visible:bg-gray-100 focus-visible:dark:text-gray-100 dark:focus-visible:bg-gray-900 text-gray-500 dark:text-gray-300 dark:hover:text-gray-400 hover:text-gray-700 focus-visible:text-gray-700 focus-visible:ring-gray-300',
			contained:
				'bg-gray-500 hover:bg-gray-700 focus-visible:bg-gray-700 text-white focus-visible:ring-gray-300',
			divider: 'divide-x divide-gray-600'
		},
		light: {
			border:
				'border  bg-surface hover:bg-surface-hover focus-visible:bg-surface-hover text-primary hover:text-secondary focus-visible:text-secondary focus-visible:ring-surface-selected',
			contained:
				'bg-surface border-transparent hover:bg-surface-hover focus-visible:bg-surface-hover text-primary focus-visible:ring-surface-selected',
			divider: 'divide-x divide-gray-200 dark:divide-gray-700'
		},
		nord: {
			border:
				'border-nord-200 bg-surface hover:bg-surface-hover focus-visible:bg-surface-hover text-primary hover:text-secondary focus-visible:text-secondary focus-visible:ring-surface-selected-inverse dark:border-nord-200',
			contained:
				'bg-nord-300 hover:bg-nord-0 focus-visible:bg-surface-hover-inverse text-primary-inverse focus-visible:ring-surface-selected-inverse dark:bg-nord-400 dark:hover:bg-nord-600 dark:text-primary-inverse',
			divider: 'divide-x divide-gray-800 dark:divide-gray-200'
		}
	}
}

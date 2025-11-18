import type { IconType } from '$lib/utils'

export const BUTTON_COLORS = [
	'blue',
	'red',
	'dark',
	'light',
	'green',
	'gray',
	'none',
	'marine',
	'nord'
] as const

export namespace ButtonType {
	/**
	 * @deprecated Use `UnifiedSize` instead
	 */
	export type Size = 'xs3' | 'xs2' | 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	export type UnifiedSize = 'xs' | 'sm' | 'md' | 'lg'
	export type ExtendedSize = 'xs2' | 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	/**
	 * @deprecated Use `Variant` instead
	 */
	export type Color =
		| 'blue'
		| 'red'
		| 'dark'
		| 'light'
		| 'green'
		| 'gray'
		| 'none'
		| 'marine'
		| 'nord'
	export type Variant =
		| 'contained' // Deprecated
		| 'border' // Deprecated
		| 'divider' // Deprecated
		| 'accent-secondary'
		| 'accent'
		| 'default'
		| 'subtle'
	export type Target = '_self' | '_blank'
	export type Element = HTMLButtonElement | HTMLAnchorElement
	export interface Icon {
		icon?: IconType | undefined
		classes?: string
		faIcon?: any | undefined
		props?: any
	}

	export const FontSizeClasses: Record<ButtonType.Size, string> = {
		xs3: 'text-2xs',
		xs2: 'text-xs',
		xs: 'text-xs',
		sm: 'text-xs',
		md: 'text-xs',
		lg: 'text-xs',
		xl: 'text-xs'
	} as const

	export const SpacingClasses: Record<
		ButtonType.Size,
		Record<'contained' | 'border' | 'divider', string>
	> = {
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

	export const ColorVariants: Record<
		ButtonType.Color,
		Record<'contained' | 'border' | 'divider', string>
	> = {
		none: {
			border: '',
			contained: '',
			divider: ''
		},

		blue: {
			border:
				'border-blue-500 dark:border-blue-300 hover:border-blue-700 focus-visible:border-blue-700 bg-surface hover:bg-blue-100 dark:hover:bg-blue-900/40 focus-visible:bg-blue-100 focus-visible:dark:text-blue-100 dark:focus-visible:bg-blue-900 text-blue-500 dark:text-blue-300 dark:hover:text-blue-200 hover:text-blue-700 focus-visible:text-blue-700 focus-visible:ring-blue-300',
			contained:
				'bg-blue-300 hover:bg-blue-400 focus-visible:bg-blue-500 text-white focus-visible:ring-blue-300',
			divider: 'divide-x divide-blue-600'
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

	export const VariantStyles: Record<'accent-secondary' | 'accent' | 'default' | 'subtle', string> =
		{
			'accent-secondary':
				'bg-surface-accent-secondary hover:bg-surface-accent-secondary-hover focus-visible:bg-surface-accent-secondary-clicked text-white dark:text-deep-blue-900 focus-visible:ring-blue-300',
			accent:
				'bg-surface-accent-primary hover:bg-surface-accent-hover focus-visible:bg-surface-accent-clicked text-white focus-visible:ring-blue-300',
			default:
				'border border-border-light bg-transparent hover:bg-surface-hover text-primary focus-visible:bg-surface-hover focus-visible:ring-blue-300',
			subtle:
				'bg-transparent hover:bg-surface-hover text-primary focus-visible:bg-surface-hover focus-visible:ring-blue-300'
		}

	export const DestructiveVariantStyles: Record<
		'accent-secondary' | 'accent' | 'default' | 'subtle',
		string
	> = {
		'accent-secondary':
			'bg-red-600 hover:bg-red-700 focus-visible:bg-red-700 text-white focus-visible:ring-red-300',
		accent:
			'bg-red-500 dark:bg-red-600 hover:bg-red-600 dark:hover:bg-red-700 focus-visible:bg-red-700 text-white focus-visible:ring-red-300',
		default:
			'border border-border-light bg-transparent hover:bg-red-500 dark:hover:bg-red-600 hover:text-white dark:hover:bg-red-600 text-primary focus-visible:bg-red-100 dark:focus-visible:bg-red-900/30 focus-visible:ring-red-300',
		subtle:
			'bg-transparent hover:bg-red-500 hover:text-white dark:hover:bg-red-600 text-primary focus-visible:bg-red-100 dark:focus-visible:bg-red-900/30 focus-visible:ring-red-300'
	}

	export const VariantSpacingClasses: Record<ButtonType.Size, string> = {
		xs3: 'px-0.5 py-[1px]',
		xs2: 'px-2 py-[4px]',
		xs: 'px-3 py-[7px]',
		sm: 'px-3 py-[7px]',
		md: 'px-3 py-[7px]',
		lg: 'px-4 py-[9px]',
		xl: 'px-4 py-[9px]'
	}

	export const IconOnlyVariantSpacingClasses: Record<ButtonType.Size, string> = {
		xs3: 'px-0.5 py-[1px]',
		xs2: 'px-1 py-[4px]',
		xs: 'px-2 py-[7px]',
		sm: 'px-2 py-[7px]',
		md: 'px-2 py-[7px]',
		lg: 'px-2.5 py-[9px]',
		xl: 'px-2.5 py-[9px]'
	}

	// New unified sizing system
	export const UnifiedSizingClasses: Record<ButtonType.UnifiedSize, string> = {
		xs: 'px-1',
		sm: 'px-2', // Regular horizontal padding
		md: 'px-4',
		lg: 'px-6'
	}

	export const UnifiedIconOnlySizingClasses: Record<ButtonType.UnifiedSize, string> = {
		xs: 'px-1',
		sm: 'px-2', // Square padding for icon-only (same as width padding)
		md: 'px-2',
		lg: 'px-4'
	}

	export const UnifiedMinHeightClasses: Record<ButtonType.UnifiedSize, string> = {
		xs: 'min-h-5',
		sm: 'min-h-7',
		md: 'min-h-8',
		lg: 'min-h-10'
	}

	export const UnifiedHeightClasses: Record<ButtonType.UnifiedSize, string> = {
		xs: 'h-5',
		sm: 'h-7',
		md: 'h-8',
		lg: 'h-10'
	}

	export const UnifiedIconSizes: Record<ButtonType.UnifiedSize, number> = {
		xs: 12,
		sm: 13,
		md: 14,
		lg: 18
	}

	export const UnifiedFontSizes: Record<ButtonType.UnifiedSize, string> = {
		xs: 'font-normal',
		sm: 'font-normal',
		md: 'font-medium',
		lg: 'font-medium'
	}

	// Extended sizing system for App editor with increased padding and icon sizes
	export const ExtendedSizingClasses: Record<ButtonType.ExtendedSize, string> = {
		xs2: 'px-1', // Smallest
		xs: 'px-2', // Very small
		sm: 'px-3', // Small (increased from unified)
		md: 'px-5', // Medium (increased from unified)
		lg: 'px-7', // Large (increased from unified)
		xl: 'px-9' // Extra large (new)
	}

	export const ExtendedIconOnlySizingClasses: Record<ButtonType.ExtendedSize, string> = {
		xs2: 'px-1', // Square padding for icon-only
		xs: 'px-2',
		sm: 'px-3',
		md: 'px-5',
		lg: 'px-7',
		xl: 'px-9'
	}

	export const ExtendedHeightClasses: Record<ButtonType.ExtendedSize, string> = {
		xs2: 'min-h-5', // 20px
		xs: 'min-h-6', // 24px
		sm: 'min-h-7', // 28px
		md: 'min-h-9', // 36px (increased from unified)
		lg: 'min-h-12', // 48px (increased from unified)
		xl: 'min-h-14' // 56px (new)
	}

	export const ExtendedIconSizes: Record<ButtonType.ExtendedSize, number> = {
		xs2: 10, // Smallest icons
		xs: 12,
		sm: 14,
		md: 16, // Increased from unified
		lg: 20, // Increased from unified
		xl: 24 // Largest icons
	}

	export const ExtendedFontSizeClasses: Record<ButtonType.ExtendedSize, string> = {
		xs2: 'text-2xs',
		xs: 'text-xs',
		sm: 'text-xs',
		md: 'text-xs',
		lg: 'text-sm',
		xl: 'text-sm'
	}
}

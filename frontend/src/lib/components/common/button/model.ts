export const BUTTON_COLORS = ['blue', 'red', 'dark', 'light', 'green', 'gray', 'none'] as const

export namespace ButtonType {
	export type Size = 'xs2' | 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	export type Color = (typeof BUTTON_COLORS)[number]
	export type Variant = 'contained' | 'border' | 'divider'
	export type Target = '_self' | '_blank'
	export type Element = HTMLButtonElement | HTMLAnchorElement
	export interface Icon {
		icon?: any | undefined
		classes?: string
		faIcon?: any | undefined
	}

	export const FontSizeClasses: Record<ButtonType.Size, string> = {
		xs2: 'text-xs',
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	} as const

	export const SpacingClasses: Record<ButtonType.Size, Record<ButtonType.Variant, string>> = {
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

	export const IconScale: Record<ButtonType.Size, number> = {
		xs2: 0.6,
		xs: 0.7,
		sm: 0.8,
		md: 1,
		lg: 1.1,
		xl: 1.2
	} as const

	export interface ItemProps {
		size: Size
		color: Color
	}
}

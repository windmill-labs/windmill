export const BUTTON_COLORS = ['blue', 'red', 'dark', 'light', 'green', 'gray', 'none'] as const

export namespace ButtonType {
	export type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	export type Color = (typeof BUTTON_COLORS)[number]
	export type Variant = 'contained' | 'border'
	export type Target = '_self' | '_blank'
	export type Element = HTMLButtonElement | HTMLAnchorElement
	export interface Icon {
		icon: any
		classes?: string
	}

	export const FontSizeClasses: Record<ButtonType.Size, string> = {
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	} as const

	export const SpacingClasses: Record<ButtonType.Size, Record<ButtonType.Variant, string>> = {
		xs: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]'
		},
		sm: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]'
		},
		md: {
			border: 'px-3 py-[6px]',
			contained: 'px-3 py-[7px]'
		},
		lg: {
			border: 'px-4 py-[8px]',
			contained: 'px-4 py-[9px]'
		},
		xl: {
			border: 'px-4 py-[8px]',
			contained: 'px-4 py-[9px]'
		}
	} as const

	export const IconScale: Record<ButtonType.Size, number> = {
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

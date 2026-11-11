type BaseColor = 'blue' | 'gray' | 'red' | 'green' | 'yellow' | 'indigo' | 'orange' | 'violet'
export const ColorModifier = 'dark-'
/** `transparent` is outlined; `subtle` has neither fill nor border. */
export type BadgeColor =
	| BaseColor
	| 'transparent'
	| 'subtle'
	| `${typeof ColorModifier}${BaseColor}`

export interface BadgeIconProps {
	position?: 'left' | 'right'
	icon: any
}

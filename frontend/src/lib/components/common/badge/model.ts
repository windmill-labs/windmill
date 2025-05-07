type BaseColor = 'blue' | 'gray' | 'red' | 'green' | 'yellow' | 'indigo' | 'orange' | 'violet'
export const ColorModifier = 'dark-'
export type BadgeColor = BaseColor | `${typeof ColorModifier}${BaseColor}`

export interface BadgeIconProps {
	position?: 'left' | 'right'
	icon: any
}

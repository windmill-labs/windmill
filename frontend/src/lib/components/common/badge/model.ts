import type { IconProps } from 'svelte-awesome/components/Icon.svelte'

type BaseColor = 'blue' | 'gray' | 'red' | 'green' | 'yellow' | 'indigo'
export const ColorModifier = 'dark-'
export type BadgeColor = BaseColor | `${typeof ColorModifier}${BaseColor}`

export interface BadgeIconProps extends IconProps {
	position?: 'left' | 'right'
}

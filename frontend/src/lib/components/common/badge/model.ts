type BaseColor = 'blue' | 'gray' | 'red' | 'green' | 'yellow' | 'indigo' | 'purple' | 'pink'
export const ColorModifier = 'dark-'
export type BadgeColor = BaseColor | `${typeof ColorModifier}${BaseColor}`

const SIDE = ['auto', 'top', 'bottom', 'left', 'right'] as const
const ALIGN = ['start', 'end'] as const
export type PopoverPlacement = `${typeof SIDE[number]}` | `${typeof SIDE[number]}-${typeof ALIGN[number]}`
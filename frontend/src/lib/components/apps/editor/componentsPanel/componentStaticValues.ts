import { BUTTON_COLORS } from '../../../common'

const buttonColorOptions = [...BUTTON_COLORS]

export const staticValues = {
	buttonColorOptions,
	buttonSizeOptions: ['xs', 'sm', 'md', 'lg', 'xl']
} as const

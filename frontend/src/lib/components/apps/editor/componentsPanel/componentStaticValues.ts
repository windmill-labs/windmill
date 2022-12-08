import { BUTTON_COLORS } from '../../../common'

const buttonColorOptions = [...BUTTON_COLORS]

export const staticValues = {
	buttonColorOptions,
	buttonSizeOptions: ['xs', 'sm', 'md', 'lg', 'xl'],
	tableSearchOptions: ['Frontend', 'Backend', 'Disabled'],
	chartThemeOptions: ['theme1', 'theme2', 'theme3']
} as const

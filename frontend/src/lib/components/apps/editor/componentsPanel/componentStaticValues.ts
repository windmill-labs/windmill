import { BUTTON_COLORS } from '../../../common'

const buttonColorOptions = [...BUTTON_COLORS]

export const staticValues = {
	buttonColorOptions,
	buttonSizeOptions: ['xs', 'sm', 'md', 'lg', 'xl'],
	tableSearchOptions: ['By Component', 'By Runnable', 'Disabled'],
	chartThemeOptions: ['theme1', 'theme2', 'theme3'],
	textStyleOptions: ['Title', 'Subtitle', 'Body', 'Label', 'Caption'],
	currencyOptions: ['USD', 'EUR', 'GBP', 'CAD', 'AUD', 'JPY', 'CNY', 'INR', 'BRL'],
	localeOptions: [
		'en-US',
		'en-GB',
		'en-IE',
		'de-DE',
		'fr-FR',
		'br-FR',
		'ja-JP',
		'pt-TL',
		'fr-CA',
		'en-CA'
	],
	objectFitOptions: ['contain', 'cover', 'fill'],
	splitPanesOrientationOptions: ['horizontal', 'vertical']
} as const

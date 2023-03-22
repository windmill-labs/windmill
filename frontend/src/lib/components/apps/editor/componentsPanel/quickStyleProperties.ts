import { writable } from 'svelte/store'
import {
	AlignCenter,
	AlignJustify,
	AlignLeft,
	AlignRight,
	Asterisk,
	Eye,
	EyeOff,
	Grid,
	Italic,
	Mouse,
	MousePointer,
	Pointer,
	RectangleHorizontal,
	Slash,
	Square,
	StretchVertical,
	Strikethrough,
	Type,
	Underline
} from 'lucide-svelte'
import type { AppCssItemName } from '../../types'

export const STYLE_STORE_KEY = 'style_store' as const

export type StyleStore = ReturnType<typeof createStyleStore>
export type StyleStoreValue = {
	style: {
		prop: (typeof StyleProperty)[number]
		value: string | undefined
	}[]
	topColors: TopColors
}

export function createStyleStore(properties: StylePropertyKey[]) {
	const style = StyleProperty.filter((p) => properties.includes(p.key)).map((p) => ({
		prop: p,
		value: '' as string | undefined
	}))
	const store = writable<StyleStoreValue>({
		style: [...style],
		topColors: [] as TopColors
	})

	return {
		subscribe: store.subscribe,
		set: store.set,
		update: store.update,
		updateProp: (key: StylePropertyKey, value?: string) => {
			if (!key) {
				return
			}
			store.update((s) => {
				const index = s.style.findIndex((p) => p.prop.key === key)
				if (index !== -1) {
					s.style[index].value = value || ''
				}
				return s
			})
		},
		getProp(key: StylePropertyKey) {
			return style.find((p) => p.prop.key === key)
		},
		resetStyle: () => {
			store.update((s) => {
				s.style = style.map((s) => ({ ...s, value: '' }))
				return s
			})
		},
		setTopColors: (colors: TopColors) => {
			store.update((s) => {
				s.topColors = colors
				return s
			})
		}
	}
}

export enum StylePropertyType {
	'color', // color value
	'unit', // number with unit like px, em, rem, etc.
	'number', // bare number like the value of 'font-weight'
	'text' // text like the value of 'display'
}

export const StylePropertyUnits = ['px', 'em', 'rem', '%', 'vh', 'vw']

export type TopColors = [] | [string] | [string, string] | [string, string, string]

export type StylePropertyKey = (typeof StyleProperty)[number]['key']

// Using an array instead of an object to preserve the order of the properties
export const StyleProperty = [
	{
		key: 'display',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'block',
					icon: Square
				},
				{
					text: 'inline-block',
					icon: RectangleHorizontal
				},
				{
					text: 'flex',
					icon: StretchVertical
				},
				{
					text: 'grid',
					icon: Grid
				}
			]
		}
	},
	{
		key: 'padding',
		value: [
			{
				type: StylePropertyType.unit,
				title: 'top'
			},
			{
				type: StylePropertyType.unit,
				title: 'right'
			},
			{
				type: StylePropertyType.unit,
				title: 'bottom'
			},
			{
				type: StylePropertyType.unit,
				title: 'left'
			}
		]
	},
	// 'margin'
	// 'box-shadow'
	{
		key: 'opacity',
		value: {
			type: StylePropertyType.number
		}
	},
	{
		key: 'cursor',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'default',
					icon: MousePointer
				},
				{
					text: 'pointer',
					icon: Pointer
				}
			]
		}
	},
	{
		key: 'width',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'min-width',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'max-width',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'height',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'min-height',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'max-height',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'border',
		value: [
			{
				type: StylePropertyType.unit,
				title: 'width'
			},
			{
				type: StylePropertyType.text,
				title: 'style',
				options: [
					{
						text: 'solid',
						icon: '___'
					},
					{
						text: 'dashed',
						icon: '_ _'
					},
					{
						text: 'dotted',
						icon: '. .'
					}
				]
			},
			{
				type: StylePropertyType.color,
				title: 'color'
			}
		]
	},
	{
		key: 'border-radius',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'background-color',
		value: {
			type: StylePropertyType.color
		}
	},
	{
		key: 'color',
		value: {
			type: StylePropertyType.color
		}
	},
	{
		key: 'font-size',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'font-family',
		value: {
			type: StylePropertyType.text
		}
	},
	{
		key: 'font-weight',
		value: {
			type: StylePropertyType.number
		}
	},
	{
		key: 'font-style',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'normal',
					icon: Type
				},
				{
					text: 'italic',
					icon: Italic
				}
			]
		}
	},
	{
		key: 'text-align',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'left',
					icon: AlignLeft
				},
				{
					text: 'center',
					icon: AlignCenter
				},
				{
					text: 'right',
					icon: AlignRight
				},
				{
					text: 'justify',
					icon: AlignJustify
				}
			]
		}
	},
	{
		key: 'text-decoration',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'none',
					icon: Slash
				},
				{
					text: 'underline',
					icon: Underline
				},
				{
					text: 'line-through',
					icon: Strikethrough
				}
			]
		}
	},
	{
		key: 'text-transform',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'none',
					icon: Slash
				},
				{
					text: 'capitalize',
					icon: 'Aa'
				},
				{
					text: 'uppercase',
					icon: 'AA'
				},
				{
					text: 'lowercase',
					icon: 'aa'
				}
			]
		}
	},
	{
		key: 'line-height',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'letter-spacing',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'word-spacing',
		value: {
			type: StylePropertyType.unit
		}
	},
	{
		key: 'overflow',
		value: {
			type: StylePropertyType.text,
			options: [
				{
					text: 'auto',
					icon: Asterisk
				},
				{
					text: 'visible',
					icon: Eye
				},
				{
					text: 'hidden',
					icon: EyeOff
				},
				{
					text: 'scroll',
					icon: Mouse
				}
			]
		}
	}
	// 'white-space'
	// 'transition'
	// 'scroll-behavior':
] as const

export const quickStyleProperties: Record<AppCssItemName, StylePropertyKey[]> = {
	grid: [
		'background-color',
		'color',
		'font-size',
		'font-family',
		'font-weight',
		'border',
		'border-radius',
		'padding',
		'width',
		'min-width',
		'max-width',
		'height',
		'min-height',
		'max-height',
		'overflow',
		'display'
	],
	viewer: [],
	mapcomponent: [],
	pdfcomponent: [],
	formcomponent: [],
	htmlcomponent: [],
	iconcomponent: [],
	tabscomponent: [],
	textcomponent: [],
	imagecomponent: [],
	rangecomponent: [],
	tablecomponent: [],
	aggridcomponent: [],
	buttoncomponent: [],
	drawercomponent: [],
	plotlycomponent: [],
	selectcomponent: [],
	slidercomponent: [],
	displaycomponent: [],
	barchartcomponent: [],
	checkboxcomponent: [],
	currencycomponent: [],
	piechartcomponent: [],
	vegalitecomponent: [],
	containercomponent: [
		'background-color',
		'color',
		'font-size',
		'font-family',
		'font-weight',
		'border',
		'border-radius',
		'padding',
		'width',
		'min-width',
		'max-width',
		'height',
		'min-height',
		'max-height',
		'overflow',
		'display'
	],
	dateinputcomponent: [],
	fileinputcomponent: [],
	textinputcomponent: [],
	emailinputcomponent: [],
	formbuttoncomponent: [],
	timeseriescomponent: [],
	multiselectcomponent: [],
	numberinputcomponent: [],
	scatterchartcomponent: [],
	passwordinputcomponent: [],
	resourceselectcomponent: [],
	verticaldividercomponent: [],
	horizontaldividercomponent: [],
	verticalsplitpanescomponent: [],
	horizontalsplitpanescomponent: []
}

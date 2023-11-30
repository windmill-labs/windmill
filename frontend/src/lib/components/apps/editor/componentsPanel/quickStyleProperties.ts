import { get, writable } from 'svelte/store'
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
import type { components } from '../component'

export const STYLE_STORE_KEY = 'style_store' as const

export type StyleStore = ReturnType<typeof createStyleStore>
export type StyleStoreValue = {
	style: {
		prop: (typeof StyleProperty)[number]
		value: string | undefined
	}[]
	topColors: TopColors
}

export function createStyleStore(properties: PropertyGroup[]) {
	const propertyNames = properties.reduce((acc, p) => {
		Object.values(p).forEach((names) => acc.push(...names))
		return acc
	}, [] as string[])
	const style = StyleProperty.filter((p) => propertyNames.includes(p.key)).map((p) => ({
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
		updatePropValue: (key: StylePropertyKey, value?: string) => {
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
			const s = get(store)
			const index = s.style.findIndex((p) => p.prop.key === key)
			return {
				prop: s.style[index],
				index: index === -1 ? undefined : index
			}
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
	'color' = 'color', // color value
	'unit' = 'unit', // number with unit like px, em, rem, etc.
	'number' = 'number', // bare number like the value of 'font-weight'
	'text' = 'text' // text like the value of 'display'
}

export const StylePropertyUnits = ['px', 'em', 'rem', '%', 'vh', 'vw']

export type TopColors = [] | [string] | [string, string] | [string, string, string]

export type StylePropertyOption = {
	text: string
	icon: string | any
}

export type BaseStylePropertyValue<T extends StylePropertyType> = {
	type: T
	title?: string
}

export type StylePropertyColorValue = BaseStylePropertyValue<StylePropertyType.color>

export type StylePropertyUnitValue = BaseStylePropertyValue<StylePropertyType.unit>

export type StylePropertyNumberValue = BaseStylePropertyValue<StylePropertyType.number> & {
	step?: number
	min?: number
	max?: number
}

export type StylePropertyTextValue = BaseStylePropertyValue<StylePropertyType.text> & {
	options?: StylePropertyOption[]
}

export type StylePropertyValue =
	| StylePropertyColorValue
	| StylePropertyUnitValue
	| StylePropertyNumberValue
	| StylePropertyTextValue

export type StyleProperty = {
	key: string
	value: StylePropertyValue | StylePropertyValue[]
}

export type StylePropertyKey = (typeof StyleProperty)[number]['key']

// Using an array instead of an object to preserve the order of the properties
export const StyleProperty: StyleProperty[] = [
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
	{
		key: 'opacity',
		value: {
			type: StylePropertyType.number,
			step: 0.1,
			min: 0,
			max: 1
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
		key: 'box-shadow',
		value: [
			{
				type: StylePropertyType.unit,
				title: 'offset-x'
			},
			{
				type: StylePropertyType.unit,
				title: 'offset-y'
			},
			{
				type: StylePropertyType.unit,
				title: 'blur'
			},
			{
				type: StylePropertyType.unit,
				title: 'spread'
			},
			{
				type: StylePropertyType.color,
				title: 'color'
			}
		]
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
		key: 'font-family',
		value: {
			type: StylePropertyType.text
		}
	},
	{
		key: 'font-size',
		value: {
			type: StylePropertyType.unit
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
]

export type PropertyGroup = Record<string, StylePropertyKey[]>

// Property groups
const layoutGrouping: PropertyGroup = {
	layout: ['display', 'overflow']
}

const sizeGrouping: PropertyGroup = {
	size: ['width', 'min-width', 'max-width', 'height', 'min-height', 'max-height']
}

const spacingGrouping: PropertyGroup = {
	spacing: ['padding']
}

const borderGrouping: PropertyGroup = {
	borders: ['border', 'border-radius']
}

const typographyGrouping: PropertyGroup = {
	typography: [
		'color',
		'font-size',
		'font-family',
		'font-weight',
		'font-style',
		'text-align',
		'text-decoration',
		'text-transform',
		'line-height',
		'letter-spacing',
		'word-spacing'
	]
}

const backgroundGrouping: PropertyGroup = {
	background: ['background-color', 'opacity', 'box-shadow']
}

const miscGrouping: PropertyGroup = {
	miscellaneous: ['cursor']
}

// Commonly used-together property groups
const containerDefaultProps: PropertyGroup[] = [
	layoutGrouping,
	spacingGrouping,
	backgroundGrouping,
	borderGrouping
]

const buttonDefaultProps: PropertyGroup[] = [
	backgroundGrouping,
	borderGrouping,
	typographyGrouping,
	miscGrouping
]

const inputDefaultProps: PropertyGroup[] = [borderGrouping, typographyGrouping, miscGrouping]

const sliderDefaultProps: PropertyGroup[] = [
	{
		colors: ['color', 'background-color', 'opacity']
	},
	borderGrouping,
	miscGrouping
]

const dividerDefaultProps: PropertyGroup[] = [
	{
		colors: ['background-color', 'opacity']
	},
	sizeGrouping,
	spacingGrouping,
	borderGrouping
]

export const quickStyleProperties: Record<
	keyof typeof components,
	Record<string, PropertyGroup[]>
> = {
	mapcomponent: {
		map: [spacingGrouping, borderGrouping]
	},
	steppercomponent: {
		container: containerDefaultProps
	},
	conditionalwrapper: {
		container: containerDefaultProps
	},
	pdfcomponent: {
		container: containerDefaultProps
	},
	formcomponent: {
		container: containerDefaultProps,
		button: buttonDefaultProps
	},
	customcomponent: {},
	htmlcomponent: {
		container: [
			layoutGrouping,
			sizeGrouping,
			spacingGrouping,
			borderGrouping,
			typographyGrouping,
			backgroundGrouping
		]
	},
	mardowncomponent: {
		container: containerDefaultProps
	},
	iconcomponent: {
		container: containerDefaultProps,
		icon: [
			{
				colors: ['color', 'background-color', 'opacity']
			},
			sizeGrouping,
			spacingGrouping,
			miscGrouping
		]
	},
	tabscomponent: {
		tabRow: containerDefaultProps,
		allTabs: [typographyGrouping, sizeGrouping],
		selectedTab: [typographyGrouping, sizeGrouping],
		container: containerDefaultProps
	},
	textcomponent: {
		text: [typographyGrouping]
	},
	imagecomponent: {
		image: containerDefaultProps
	},
	rangecomponent: {
		handles: sliderDefaultProps,
		bar: sliderDefaultProps,
		limits: [typographyGrouping],
		values: [typographyGrouping]
	},
	tablecomponent: {
		tableHeader: containerDefaultProps,
		tableBody: containerDefaultProps,
		tableFooter: containerDefaultProps,
		container: containerDefaultProps
	},
	aggridcomponent: {},
	aggridcomponentee: {},
	buttoncomponent: {
		button: buttonDefaultProps
	},
	downloadcomponent: {
		button: buttonDefaultProps
	},
	drawercomponent: {
		container: containerDefaultProps
	},
	modalcomponent: {
		container: containerDefaultProps
	},
	plotlycomponent: {},
	plotlycomponentv2: {},
	selectcomponent: {
		input: inputDefaultProps
	},
	slidercomponent: {
		handles: sliderDefaultProps,
		bar: sliderDefaultProps,
		limits: [typographyGrouping],
		values: [typographyGrouping]
	},
	displaycomponent: {
		header: [...containerDefaultProps, typographyGrouping],
		container: containerDefaultProps
	},
	logcomponent: {
		header: [...containerDefaultProps, typographyGrouping],
		container: containerDefaultProps
	},
	jobidlogcomponent: {
		header: [...containerDefaultProps, typographyGrouping],
		container: containerDefaultProps
	},
	carousellistcomponent: {
		container: containerDefaultProps
	},
	flowstatuscomponent: {
		header: [...containerDefaultProps, typographyGrouping],
		container: containerDefaultProps
	},
	jobidflowstatuscomponent: {
		header: [...containerDefaultProps, typographyGrouping],
		container: containerDefaultProps
	},
	barchartcomponent: {
		container: containerDefaultProps
	},
	checkboxcomponent: {
		text: [typographyGrouping]
	},
	currencycomponent: {
		input: inputDefaultProps
	},
	piechartcomponent: {
		container: containerDefaultProps
	},
	chartjscomponent: {
		container: containerDefaultProps
	},
	chartjscomponentv2: {
		container: containerDefaultProps
	},
	vegalitecomponent: {},
	containercomponent: {
		container: containerDefaultProps
	},
	listcomponent: {
		container: containerDefaultProps
	},
	dateinputcomponent: {
		input: inputDefaultProps
	},
	fileinputcomponent: {
		container: containerDefaultProps
	},
	textinputcomponent: {
		input: inputDefaultProps
	},
	quillcomponent: {
		input: inputDefaultProps
	},
	textareainputcomponent: {
		input: inputDefaultProps
	},
	emailinputcomponent: {
		input: inputDefaultProps
	},
	formbuttoncomponent: {
		button: buttonDefaultProps,
		popup: containerDefaultProps
	},
	timeseriescomponent: {
		container: containerDefaultProps
	},
	multiselectcomponent: {
		input: inputDefaultProps
	},
	numberinputcomponent: {
		input: inputDefaultProps
	},
	scatterchartcomponent: {
		container: containerDefaultProps
	},
	passwordinputcomponent: {
		input: inputDefaultProps
	},
	resourceselectcomponent: {
		input: inputDefaultProps
	},
	verticaldividercomponent: {
		divider: dividerDefaultProps,
		container: containerDefaultProps
	},
	horizontaldividercomponent: {
		divider: dividerDefaultProps,
		container: containerDefaultProps
	},
	verticalsplitpanescomponent: {
		container: containerDefaultProps
	},
	horizontalsplitpanescomponent: {
		container: containerDefaultProps
	},
	schemaformcomponent: {
		container: containerDefaultProps,
		button: buttonDefaultProps
	},
	selecttabcomponent: {
		tabRow: containerDefaultProps,
		allTabs: [typographyGrouping, sizeGrouping],
		selectedTab: [typographyGrouping, sizeGrouping]
	},
	selectstepcomponent: {},
	statcomponent: {},
	menucomponent: {
		button: buttonDefaultProps
	}
}

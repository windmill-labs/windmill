import type { ComponentSet } from '../../types'

const layout: ComponentSet = {
	title: 'Layout',
	components: [
		'tabscomponent',
		'containercomponent',
		'horizontaldividercomponent',
		'verticaldividercomponent',
		'drawercomponent'
	]
} as const

const buttons: ComponentSet = {
	title: 'Buttons',
	components: ['buttoncomponent', 'formcomponent', 'formbuttoncomponent']
} as const

const inputs: ComponentSet = {
	title: 'Inputs',
	components: [
		'textinputcomponent',
		'passwordinputcomponent',
		'numberinputcomponent',
		'currencycomponent',
		'slidercomponent',
		'rangecomponent',
		'dateinputcomponent',
		'fileinputcomponent',
		'checkboxcomponent',
		'selectcomponent'
	]
} as const

const display: ComponentSet = {
	title: 'Display',
	components: [
		'textcomponent',
		'iconcomponent',
		'imagecomponent',
		'htmlcomponent',
		'tablecomponent',
		'aggridcomponent',
		'barchartcomponent',
		'piechartcomponent',
		'vegalitecomponent',
		'plotlycomponent',
		'scatterchartcomponent',
		'timeseriescomponent',
		'displaycomponent'
	]
} as const

export const COMPONENT_SETS = [layout, buttons, inputs, display] as const

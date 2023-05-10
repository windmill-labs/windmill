import type { ComponentSet } from '../../types'

const tabs: ComponentSet = {
	title: 'Tabs',
	components: ['tabscomponent', 'conditionalwrapper']
} as const

const layout: ComponentSet = {
	title: 'Layout',
	components: [
		'containercomponent',
		'horizontaldividercomponent',
		'verticaldividercomponent',
		'drawercomponent',
		'verticalsplitpanescomponent',
		'horizontalsplitpanescomponent',
		'modalcomponent',
		'steppercomponent'
	]
} as const

const buttons: ComponentSet = {
	title: 'Buttons',
	components: ['buttoncomponent', 'formcomponent', 'formbuttoncomponent']
} as const

const inputs: ComponentSet = {
	title: 'Inputs',
	components: [
		'schemaformcomponent',
		'textinputcomponent',
		'textareainputcomponent',
		'passwordinputcomponent',
		'emailinputcomponent',
		'numberinputcomponent',
		'currencycomponent',
		'slidercomponent',
		'rangecomponent',
		'dateinputcomponent',
		'fileinputcomponent',
		'checkboxcomponent',
		'selectcomponent',
		'resourceselectcomponent',
		'multiselectcomponent'
	]
} as const

const display: ComponentSet = {
	title: 'Display',
	components: [
		'textcomponent',
		'iconcomponent',
		'imagecomponent',
		'mapcomponent',
		'htmlcomponent',
		'pdfcomponent',
		'displaycomponent'
	]
} as const

const tables: ComponentSet = {
	title: 'Tables',
	components: ['tablecomponent', 'aggridcomponent']
} as const

const charts: ComponentSet = {
	title: 'Charts',
	components: [
		'barchartcomponent',
		'piechartcomponent',
		'vegalitecomponent',
		'plotlycomponent',
		'scatterchartcomponent',
		'timeseriescomponent'
	]
} as const

export const COMPONENT_SETS = [tabs, layout, buttons, inputs, tables, display, charts] as const

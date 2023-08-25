import type { ComponentSet } from '../../types'

const tabs: ComponentSet = {
	title: 'Tabs',
	components: ['tabscomponent', 'conditionalwrapper'],
	presets: ['sidebartabscomponent', 'invisibletabscomponent']
} as const

const layout: ComponentSet = {
	title: 'Layout',
	components: [
		'containercomponent',
		'listcomponent',
		'horizontaldividercomponent',
		'verticaldividercomponent',
		'drawercomponent',
		'verticalsplitpanescomponent',
		'horizontalsplitpanescomponent',
		'modalcomponent',
		'steppercomponent',
		'carousellistcomponent'
	]
} as const

const buttons: ComponentSet = {
	title: 'Buttons',
	components: ['buttoncomponent', 'formcomponent', 'formbuttoncomponent', 'downloadcomponent']
} as const

const inputs: ComponentSet = {
	title: 'Inputs',
	components: [
		'schemaformcomponent',
		'textinputcomponent',
		'textareainputcomponent',
		'quillcomponent',
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
		'multiselectcomponent',
		'selecttabcomponent',
		'selectstepcomponent'
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
		'mardowncomponent',
		'pdfcomponent',
		'displaycomponent',
		'jobidlogcomponent',
		'jobidflowstatuscomponent',
		'customcomponent'
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
		'timeseriescomponent',
		'chartjscomponent'
	]
} as const

export const COMPONENT_SETS = [layout, tabs, buttons, inputs, tables, display, charts] as const

import type { ComponentSet } from '../../types'

const tabs: ComponentSet = {
	title: 'Tabs',
	components: ['tabscomponent', 'conditionalwrapper'],
	presets: ['sidebartabscomponent', 'accordiontabcomponent', 'invisibletabscomponent']
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
		'carousellistcomponent',
		'accordionlistcomponent',
		'decisiontreecomponent',
		'navbarcomponent',
		'recomputeallcomponent'
	],
	presets: ['topbarcomponent']
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
		'codeinputcomponent',
		'textareainputcomponent',
		'quillcomponent',
		'passwordinputcomponent',
		'emailinputcomponent',
		'numberinputcomponent',
		'currencycomponent',
		'slidercomponent',
		'dateslidercomponent',
		'rangecomponent',
		'dateinputcomponent',
		'timeinputcomponent',
		'datetimeinputcomponent',
		'dateselectcomponent',
		'fileinputcomponent',
		's3fileinputcomponent',
		'checkboxcomponent',
		'selectcomponent',
		'resourceselectcomponent',
		'userresourcecomponent',
		'multiselectcomponentv2',
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
		'jobiddisplaycomponent',
		'statcomponent',
		'menucomponent',
		'alertcomponent'
	]
} as const

const tables: ComponentSet = {
	title: 'Tables',
	components: [
		'aggridcomponent',
		'aggridcomponentee',
		'dbexplorercomponent',
		'aggridinfinitecomponent',
		'aggridinfinitecomponentee',
		'tablecomponent'
	]
} as const

const charts: ComponentSet = {
	title: 'Charts',
	components: [
		'plotlycomponentv2',
		'chartjscomponentv2',
		'vegalitecomponent',
		'agchartscomponent',
		'agchartscomponentee'
	]
} as const

export const COMPONENT_SETS = [layout, tabs, buttons, inputs, tables, display, charts] as const

export const DEPRECATED_COMPONENTS = {
	tablecomponent:
		'We will be removing this component in the future. we recommend using the AgGrid table instead.',
	chartjscomponent: 'Use the new ChartJS v2 component instead',
	multiselectcomponent: 'Use the new MultiSelect v2 component instead'
}

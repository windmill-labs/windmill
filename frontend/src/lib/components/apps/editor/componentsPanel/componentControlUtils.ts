import { components } from '../component'

export type ComponentFunction = {
	title: string
	description: string
	example: string
	documentation: string
}

const setTab = {
	title: 'setTab',
	description: 'Use the setTab function to manually set the tab of a Tab component.',
	example: 'setTab(id: string, index: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#settab'
}

const recompute = {
	title: 'recompute',
	description: 'Use the recompute function to recompute the value of a component.',
	example: 'recompute(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#recompute'
}

const getAgGrid = {
	title: 'getAgGrid',
	description: 'Use the getAgGrid function to get the ag-grid instance of a table.',
	example: 'getAgGrid(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#getaggrid'
}

const setValue = {
	title: 'setValue',
	description:
		"The setValue function is meant to set or force the value of a component. This can be convenient in cases where connection is not the easiest pattern. Note that it's a bad idea to mix dynamic default value and setValue together.",
	example: 'setValue(id: string, value: any)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#setvalue'
}

const setSelectedIndex = {
	title: 'setSelectedIndex',
	description: 'Use the setSelectedIndex function to select a row in a table or an AG Grid table.',
	example: 'setSelectedIndex(id: string, index: number)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#setselectedindex'
}

const open = {
	title: 'open',
	description: 'Use the open function to open a modal or a drawer.',
	example: 'open(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#open'
}

const close = {
	title: 'close',
	description: 'Use the close function to close a modal or a drawer.',
	example: 'close(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#close'
}

export function getComponentControl(type: keyof typeof components): Array<ComponentFunction> {
	switch (type) {
		case 'tabscomponent':
			return [setTab]
		case 'selectstepcomponent':
			return [setTab, setValue]
		case 'selecttabcomponent':
			return [setTab, setValue]
		case 'conditionalwrapper':
			return [setTab]
		case 'steppercomponent':
			return [setTab]
		case 'modalcomponent':
			return [open, close]
		case 'drawercomponent':
			return [open, close]
		case 'aggridcomponent':
			return [getAgGrid, setSelectedIndex]
		case 'aggridcomponentee':
			return [getAgGrid, setSelectedIndex]
		case 'displaycomponent':
		case 'dateinputcomponent':
		case 'textinputcomponent':
		case 'numberinputcomponent':
		case 'currencycomponent':
		case 'checkboxcomponent':
		case 'formcomponent':
		case 'rangecomponent':
		case 'multiselectcomponent':
		case 'selectcomponent':
		case 'slidercomponent':
		case 'schemaformcomponent':
		case 'quillcomponent':
		case 'textcomponent':
		case 'textareainputcomponent':
			return [setValue]
		case 'tablecomponent':
			return [setSelectedIndex]
		default:
			if (components[type].initialData['componentInput']) {
				return [recompute]
			} else {
				return []
			}
	}
}

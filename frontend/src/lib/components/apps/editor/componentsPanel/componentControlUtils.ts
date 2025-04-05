import { components } from '../component'

export type ComponentFunction = {
	title: string
	description: string
	example: string
	documentation?: string
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

const clearFiles = {
	title: 'clearFiles',
	description: 'Clear the files of a file input component.',
	example: 'clearFiles(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#clearfiles'
}

const close = {
	title: 'close',
	description: 'Use the close function to close a modal or a drawer.',
	example: 'close(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#close'
}

const validate = {
	title: 'validate',
	description: 'Validate a specific field of a form',
	example: 'validate(id: string, key: string)'
}

const invalidate = {
	title: 'invalidate',
	description: 'Invalidate a specific field of a form',
	example: 'invalidate(id: string, key: string, error: string)'
}

const validateAll = {
	title: 'validateAll',
	description: 'Validate all fields of a form',
	example: 'validateAll(id: string, key: string)'
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
		case 'steppercomponent':
			return [setTab]
		case 'modalcomponent':
			return [open, close]
		case 'drawercomponent':
			return [open, close]
		case 'aggridcomponent':
		case 'aggridcomponentee':
		case 'aggridinfinitecomponent':
		case 'aggridinfinitecomponentee':
			return [getAgGrid, setSelectedIndex]

		case 's3fileinputcomponent':
			return [clearFiles]
		case 'displaycomponent':
		case 'dateinputcomponent':
		case 'timeinputcomponent':
		case 'datetimeinputcomponent':
		case 'textinputcomponent':
		case 'numberinputcomponent':
		case 'currencycomponent':
		case 'checkboxcomponent':
		case 'rangecomponent':
		case 'multiselectcomponent':
		case 'multiselectcomponentv2':
		case 'selectcomponent':
		case 'slidercomponent':
		case 'dateslidercomponent':
		case 'quillcomponent':
		case 'textcomponent':
		case 'codeinputcomponent':
		case 'textareainputcomponent':
			return [setValue]
		case 'formcomponent':
		case 'schemaformcomponent':
		case 'formbuttoncomponent':
			return [setValue, validate, validateAll, invalidate]
		case 'tablecomponent':
			return [setSelectedIndex]
		case 'dbexplorercomponent':
			return [recompute]
		default:
			if (components[type].initialData['componentInput']) {
				return [recompute]
			} else {
				return []
			}
	}
}

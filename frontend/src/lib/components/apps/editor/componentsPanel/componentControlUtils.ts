import type { components } from '../component'

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

const openModal = {
	title: 'openModal',
	description: 'Use the openModal function to open a modal.',
	example: 'openModal(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#openmodal'
}

const closeModal = {
	title: 'closeModal',
	description: 'Use the closeModal function to close a modal.',
	example: 'closeModal(id: string)',
	documentation: 'https://www.windmill.dev/docs/apps/app-runnable-panel#closemodal'
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
			return [openModal, closeModal]
		case 'aggridcomponent':
			return [getAgGrid]
		case 'displaycomponent':
			return [setValue]
		case 'dateinputcomponent':
			return [setValue]
		case 'textinputcomponent':
			return [setValue]
		case 'numberinputcomponent':
			return [setValue]
		case 'currencycomponent':
			return [setValue]
		case 'checkboxcomponent':
			return [setValue]
		case 'formcomponent':
			return [setValue]
		case 'rangecomponent':
			return [setValue]
		case 'multiselectcomponent':
			return [setValue]
		case 'selectcomponent':
			return [setValue]
		case 'slidercomponent':
		default:
			return [recompute]
	}
}

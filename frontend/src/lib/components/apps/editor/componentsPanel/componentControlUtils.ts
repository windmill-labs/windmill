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

export function getComponentControl(type: keyof typeof components): Array<ComponentFunction> {
	switch (type) {
		case 'tabscomponent':
			return [setTab]
		case 'selectstepcomponent':
			return [setTab]
		case 'selecttabcomponent':
			return [setTab]
		case 'conditionalwrapper':
			return [setTab]
		case 'steppercomponent':
			return [setTab]
		case 'modalcomponent':
			return [openModal, closeModal]
		default:
			return [] as Array<ComponentFunction>
	}
}

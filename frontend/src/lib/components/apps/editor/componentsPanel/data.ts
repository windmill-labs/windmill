import type { AppComponent, ComponentSet } from '../../types'
import { defaultAlignement, defaultProps } from './componentDefaultProps'

const windmillComponents: ComponentSet = {
	title: 'Windmill Components',
	components: [
		{
			...defaultProps,
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInputs: {
				result: {
					id: undefined,
					name: undefined,
					type: 'output',
					defaultValue: {},
					fieldType: 'object'
				}
			}
		}
	]
}

const textInputs: ComponentSet = {
	title: 'Text Inputs',
	components: []
}

const numberInputs: ComponentSet = {
	title: 'Number Inputs',
	components: []
}

const buttons: ComponentSet = {
	title: 'Buttons',
	components: [
		{
			...defaultProps,
			...defaultAlignement,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'blue',
					optionValuesKey: 'buttonColorOptions',
					defaultValue: 'blue'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'md',
					optionValuesKey: 'buttonSizeOptions',
					defaultValue: 'md'
				}
			},
			runnable: true,
			card: false
		}
	]
}

const selectInputs: ComponentSet = {
	title: 'Select Inputs',
	components: [
		{
			...defaultProps,
			...defaultAlignement,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: undefined,
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
				}
			},
			card: false
		}
	]
}

const dateTimeInputs: ComponentSet = {
	title: 'Date and Time Inputs',
	components: []
}

const dataDisplay: ComponentSet = {
	title: 'Data Display',
	components: [
		{
			...defaultProps,
			...defaultAlignement,
			id: 'textcomponent',
			type: 'textcomponent',
			componentInputs: {
				content: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
				}
			}
		},
		{
			...defaultProps,
			id: 'tablecomponent',
			type: 'tablecomponent',
			componentInputs: {
				searchEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					visible: true,
					defaultValue: false
				},
				paginationEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					visible: true,
					defaultValue: false
				}
			},
			runnable: true,
			card: true,
			actionButtons: []
		},
		{
			...defaultProps,
			id: 'piechartcomponent',
			type: 'piechartcomponent',
			runnable: true,
			card: true
		},
		{
			...defaultProps,
			id: 'barchartcomponent',
			type: 'barchartcomponent',
			runnable: true,
			card: true
		}
	]
}

const componentSets = [
	windmillComponents,
	textInputs,
	numberInputs,
	buttons,
	selectInputs,
	dateTimeInputs,
	dataDisplay
]

export { componentSets }

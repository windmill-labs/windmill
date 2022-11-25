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
					defaultValue: undefined
				}
			}
		}
	]
}

const textInputs: ComponentSet = {
	title: 'Text Inputs',
	components: [

	]
}

const numberInputs: ComponentSet = {
	title: 'Number Inputs',
	components: [

	]
}

const buttons: ComponentSet = {
	title: 'Buttons',
	components: [
		{
			...defaultProps,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'blue',
					optionValuesKey: 'buttonColorOptions'
				}
			},
			runnable: true
		},
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
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				}
			}
		}
	]
}

const dateTimeInputs: ComponentSet = {
	title: 'Date and Time Inputs',
	components: [

	]
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
					fieldType: 'textarea'
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
					fieldType: 'boolean'
				},
				paginationEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			},
			runnable: true,
			card: true
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
		},
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

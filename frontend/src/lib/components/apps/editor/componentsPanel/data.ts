import type { ComponentSet } from '../../types'
import { defaultAlignement } from './componentDefaultProps'

const windmillComponents: ComponentSet = {
	title: 'Windmill Components',
	components: [
		{
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInput: {
				type: 'static',
				fieldType: 'text',
				defaultValue: 'Lorem Ipsum',
				value: 'Lorem Ipsum'
			},
			configuration: {},
			card: false
		}
	]
}

const textInputs: ComponentSet = {
	title: 'Text Inputs',
	components: [
		{
			id: 'textinputcomponent',
			type: 'textinputcomponent',
			componentInput: {
				type: 'static',
				fieldType: 'text',
				defaultValue: 'Lorem Ipsum',
				value: 'Lorem Ipsum'
			},
			configuration: {},
			card: false
		}
	]
}

const numberInputs: ComponentSet = {
	title: 'Number Inputs',
	components: []
}

const buttons: ComponentSet = {
	title: 'Buttons',
	components: [
		{
			...defaultAlignement,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInput: {
				type: 'static',
				fieldType: 'textarea',
				defaultValue: '',
				value: ''
			},
			recompute: undefined,
			configuration: {
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

			card: false
		}
	]
}

const selectInputs: ComponentSet = {
	title: 'Select Inputs',
	components: [
		{
			...defaultAlignement,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent',
			configuration: {
				label: {
					type: 'static',
					visible: true,
					value: undefined,
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
				}
			},
			componentInput: undefined,
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
			...defaultAlignement,
			id: 'textcomponent',
			type: 'textcomponent',
			componentInput: {
				type: 'static',
				visible: true,
				value: 'Lorem ipsum',
				fieldType: 'textarea',
				defaultValue: 'Lorem ipsum'
			},
			configuration: {},
			card: false
		},
		{
			id: 'tablecomponent',
			type: 'tablecomponent',
			configuration: {
				search: {
					fieldType: 'select',
					type: 'static',
					value: 'Disabled',
					optionValuesKey: 'tableSearchOptions',
					defaultValue: 'Disabled'
				},
				pagination: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					visible: true,
					defaultValue: false
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				defaultValue: [
					{
						id: 1,
						name: 'Lorem ipsum',
						age: 42
					},
					{
						id: 2,
						name: 'Lorem ipsum',
						age: 42
					}
				],
				value: [
					{
						id: 1,
						name: 'Lorem ipsum',
						age: 42
					},
					{
						id: 2,
						name: 'Lorem ipsum',
						age: 42
					}
				]
			},
			card: true,
			actionButtons: []
		},
		{
			id: 'piechartcomponent',
			type: 'piechartcomponent',
			configuration: {
				theme: {
					type: 'static',
					value: 'theme1',
					fieldType: 'select',
					optionValuesKey: 'chartThemeOptions',
					defaultValue: 'theme1'
				},
				labels: {
					type: 'static',
					value: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum'],
					fieldType: 'array',
					defaultValue: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum']
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				defaultValue: [25, 50, 25],
				value: [25, 50, 25]
			},
			card: true
		},
		{
			id: 'barchartcomponent',
			type: 'barchartcomponent',
			configuration: {
				theme: {
					type: 'static',
					value: 'theme1',
					fieldType: 'select',
					optionValuesKey: 'chartThemeOptions',
					defaultValue: 'theme1'
				},
				labels: {
					type: 'static',
					value: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum'],
					fieldType: 'array',
					defaultValue: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum']
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				defaultValue: [25, 50, 25],
				value: [25, 50, 25]
			},
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

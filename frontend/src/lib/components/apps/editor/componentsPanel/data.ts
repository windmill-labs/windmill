import type { ComponentSet } from '../../types'
import { defaultAlignement } from './componentDefaultProps'

const inputs: ComponentSet = {
	title: 'Inputs',
	components: [
		{
			verticalAlignment: 'center',
			id: 'textinputcomponent',
			type: 'textinputcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: false,
					value: 'Label',
					fieldType: 'textarea',
					defaultValue: 'Label'
				}
			},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'passwordinputcomponent',
			type: 'passwordinputcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: false,
					value: 'Label',
					fieldType: 'textarea',
					defaultValue: 'Label'
				}
			},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'numberinputcomponent',
			type: 'numberinputcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: false,
					value: 'Label',
					fieldType: 'textarea',
					defaultValue: 'Label'
				}
			},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'dateinputcomponent',
			type: 'dateinputcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: false,
					value: 'Title',
					fieldType: 'textarea',
					defaultValue: 'Title'
				},
				minDate: {
					type: 'static',
					visible: false,
					value: '',
					fieldType: 'date',
					defaultValue: ''
				},
				maxDate: {
					type: 'static',
					visible: false,
					value: '',
					fieldType: 'date',
					defaultValue: ''
				},
			},
			card: false,
			softWrap: true
		},
		{
			...defaultAlignement,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: true,
					value: 'Label',
					fieldType: 'textarea',
					defaultValue: 'Label'
				}
			},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'selectcomponent',
			type: 'selectcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: false,
					value: 'Label',
					fieldType: 'textarea',
					defaultValue: 'Label'
				},
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'object',
					defaultValue: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					],
					value: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					]
				},
				itemKey: {
					type: 'static',
					fieldType: 'text',
					defaultValue: 'value',
					value: 'value'
				}
			},
			card: false,
			softWrap: true
		}
	]
}

const buttons: ComponentSet = {
	title: 'Buttons',
	components: [
		{
			...defaultAlignement,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined,
				defaultValue: undefined
			},
			recomputeIds: undefined,
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
					value: 'xs',
					optionValuesKey: 'buttonSizeOptions',
					defaultValue: 'xs'
				}
			},

			card: false
		},
		{
			horizontalAlignment: 'center',
			id: 'formcomponent',
			type: 'formcomponent',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined,
				defaultValue: undefined
			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',
					visible: true,
					value: 'Submit',
					fieldType: 'textarea',
					defaultValue: 'formcomponent'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'dark',
					optionValuesKey: 'buttonColorOptions',
					defaultValue: 'dark'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'xs',
					optionValuesKey: 'buttonSizeOptions',
					defaultValue: 'xs'
				}
			},

			card: true
		}
	]
}

const display: ComponentSet = {
	title: 'Display',
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
				subFieldType: 'object',
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
					value: ['First', 'Second', 'Third'],
					fieldType: 'array',

					subFieldType: 'text',
					defaultValue: ['First', 'Second', 'Third']
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'number',
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
					subFieldType: 'text',
					defaultValue: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum']
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'number',
				defaultValue: [25, 50, 25],
				value: [25, 50, 25]
			},
			card: true
		},
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

const componentSets = [buttons, inputs, display]

export { componentSets }

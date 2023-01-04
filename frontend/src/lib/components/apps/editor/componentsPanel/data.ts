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
				placeholder: {
					type: 'static',
					value: 'Type...',
					fieldType: 'text',
					onlyStatic: true,
				},
			},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'passwordinputcomponent',
			type: 'passwordinputcomponent',
			componentInput: undefined,
			configuration: {},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'numberinputcomponent',
			type: 'numberinputcomponent',
			componentInput: undefined,
			configuration: {},
			card: false
		},
		{
			verticalAlignment: 'center',
			id: 'dateinputcomponent',
			type: 'dateinputcomponent',
			componentInput: undefined,
			configuration: {
				minDate: {
					type: 'static',
					value: '',
					fieldType: 'date',
				},
				maxDate: {
					type: 'static',
					value: '',
					fieldType: 'date',
				}
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
					value: 'Label',
					fieldType: 'text',
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
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'object',
					value: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					]
				},
				itemKey: {
					type: 'static',
					fieldType: 'text',
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
			softWrap: true,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined,

			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',

					fieldType: 'text',
					value: 'Lorem ipsum'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'buttonColorOptions',
					value: 'blue'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'buttonSizeOptions',
					value: 'xs'
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

			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',
					value: 'Submit',
					fieldType: 'text',
				},
				color: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					value: 'dark',
					optionValuesKey: 'buttonColorOptions',
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					onlyStatic: true,
					optionValuesKey: 'buttonSizeOptions',
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
			softWrap: true,
			horizontalAlignment: 'left',
			verticalAlignment: 'top',
			id: 'textcomponent',
			type: 'textcomponent',
			componentInput: {
				type: 'static',
				fieldType: 'textarea',
				value: 'Hello ${ctx.username}',
			},
			configuration: {
				style: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'textStyleOptions',
					value: 'Body'
				},
				extraStyle:
				{
					type: 'static',
					fieldType: 'text',
					value: '',

				},

			},
			card: false
		},
		{

			id: 'tablecomponent',
			type: 'tablecomponent',
			configuration: {
				search: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'tableSearchOptions',
					value: 'Disabled'
				},
				pagination: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: true
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
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
				],

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
					onlyStatic: true,
					fieldType: 'select',
					optionValuesKey: 'chartThemeOptions',
					value: 'theme1'
				},
				labels: {
					type: 'static',
					value: ['First', 'Second', 'Third'],
					fieldType: 'array',
					subFieldType: 'text',
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'number',
				value: [25, 50, 25],

			},
			card: true
		},
		{
			id: 'barchartcomponent',
			type: 'barchartcomponent',
			configuration: {
				theme: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'select',
					optionValuesKey: 'chartThemeOptions',
					value: 'theme1'
				},
				labels: {
					type: 'static',

					fieldType: 'array',
					subFieldType: 'text',
					value: ['Lorem ipsum', 'Lorem ipsum', 'Lorem ipsum']
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'number',
				value: [25, 50, 25],

			},
			card: true
		},
		{
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: { "foo": 42 },

			},
			configuration: {},
			card: false
		}
	]
}

const componentSets = [buttons, inputs, display]

export { componentSets }

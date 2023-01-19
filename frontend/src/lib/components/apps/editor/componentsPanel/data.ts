import type { ComponentSet } from '../../types'
import { defaultAlignement } from './componentDefaultProps'

const inputs: ComponentSet = {
	title: 'Inputs',
	components: [
		{
			softWrap: true,
			verticalAlignment: 'center',
			id: 'textinputcomponent',
			type: 'textinputcomponent',
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Type...',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			card: false
		},
		{
			softWrap: true,
			verticalAlignment: 'center',
			id: 'passwordinputcomponent',
			type: 'passwordinputcomponent',
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Password',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			card: false
		},
		{
			softWrap: true,
			verticalAlignment: 'center',
			id: 'numberinputcomponent',
			type: 'numberinputcomponent',
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Type...',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			card: false
		},
		{
			softWrap: true,
			verticalAlignment: 'center',
			id: 'slidercomponent',
			type: 'slidercomponent',
			componentInput: undefined,
			configuration: {
				min: {
					type: 'static',
					value: 0,
					fieldType: 'number',
					onlyStatic: true
				},
				max: {
					type: 'static',
					value: 42,
					fieldType: 'number',
					onlyStatic: true
				}
			},
			card: false
		},
		{
			softWrap: true,
			verticalAlignment: 'center',
			id: 'dateinputcomponent',
			type: 'dateinputcomponent',
			componentInput: undefined,
			configuration: {
				minDate: {
					type: 'static',
					value: '',
					fieldType: 'date'
				},
				maxDate: {
					type: 'static',
					value: '',
					fieldType: 'date'
				}
			},
			card: false
		},
		{
			...defaultAlignement,
			softWrap: true,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent',
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					value: 'Label',
					fieldType: 'text'
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
				runnable: undefined
			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
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
				},
				disabled: {
					fieldType: 'boolean',
					type: 'eval',
					expr: 'false'
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
				runnable: undefined
			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',
					value: 'Submit',
					fieldType: 'text'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					value: 'dark',
					optionValuesKey: 'buttonColorOptions'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					onlyStatic: true,
					optionValuesKey: 'buttonSizeOptions'
				}
			},

			card: true
		},
		{
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			id: 'formbuttoncomponent',
			type: 'formbuttoncomponent',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			},
			recomputeIds: undefined,
			configuration: {
				label: {
					type: 'static',
					value: 'Open popup',
					fieldType: 'text'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					value: 'dark',
					optionValuesKey: 'buttonColorOptions'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					onlyStatic: true,
					optionValuesKey: 'buttonSizeOptions'
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
			softWrap: false,
			id: 'htmlcomponent',
			type: 'htmlcomponent',
			componentInput: {
				type: 'static',
				fieldType: 'template',
				value: `<img
	src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
	Hello \${ctx.username}
</h1>`
			},
			configuration: {},
			card: false
		},
		{
			softWrap: false,
			id: 'vegalitecomponent',
			type: 'vegalitecomponent',
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: {
					data: {
						values: [
							{ a: 'A', b: 28 },
							{ a: 'B', b: 55 },
							{ a: 'C', b: 43 },
							{ a: 'D', b: 91 }
						]
					},
					mark: 'bar',
					encoding: {
						x: { field: 'a', type: 'ordinal' },
						y: { field: 'b', type: 'quantitative' }
					}
				}
			},
			configuration: {
				canvas: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false,
					tooltip: 'use the canvas renderer instead of the svg one for more interactive plots'
				}
			},
			card: false
		},
		{
			softWrap: true,
			horizontalAlignment: 'left',
			verticalAlignment: 'top',
			id: 'textcomponent',
			type: 'textcomponent',
			componentInput: {
				type: 'static',
				fieldType: 'template',
				value: 'Hello ${ctx.username}'
			},
			configuration: {
				style: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'textStyleOptions',
					value: 'Body'
				},
				extraStyle: {
					type: 'static',
					fieldType: 'text',
					value: '',
					tooltip: 'CSS rules like "color: blue;"'
				},
				copyButton: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					onlyStatic: true
				}
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
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
				value: [
					{
						id: 1,
						name: 'A cell with a long name',
						age: 42
					},
					{
						id: 2,
						name: 'A briefer cell',
						age: 84
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
					onlyStatic: true,
					fieldType: 'select',
					optionValuesKey: 'chartThemeOptions',
					value: 'theme1'
				},
				doughnutStyle: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: { data: [25, 50, 25], labels: ['Pie', 'Charts', '<3'] }
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
				line: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: { data: [25, 50, 25], labels: ['Bar', 'Charts', '<3'] }
			},
			card: true
		},
		{
			id: 'scatterchartcomponent',
			type: 'scatterchartcomponent',
			configuration: {
				zoomable: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				},
				pannable: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
				value: [
					{
						label: 'foo',
						data: [
							{ x: 25, y: 50 },
							{ x: 23, y: 23 },
							{ x: 12, y: 37 }
						],
						backgroundColor: 'rgb(255, 12, 137)'
					},
					{
						label: 'foobar',
						data: [
							{ x: 32, y: 32 },
							{ x: 25, y: 42 },
							{ x: 3, y: 27 }
						],
						backgroundColor: 'orange'
					}
				]
			},
			card: true
		},
		{
			id: 'timeseriescomponent',
			type: 'timeseriescomponent',
			configuration: {
				logarithmicScale: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				},
				zoomable: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				},
				pannable: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'boolean',
					value: false
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
				value: [
					{
						label: 'foo',
						data: [
							{
								x: '2021-11-06 23:39:30',
								y: 50
							},
							{
								x: '2021-11-07 01:00:28',
								y: 60
							},
							{
								x: '2021-11-07 09:00:28',
								y: 20
							}
						],
						backgroundColor: 'rgb(255, 12, 137)'
					},
					{
						label: 'foobar',
						data: [
							{
								x: '2021-11-06 23:39:30',
								y: 20
							},
							{
								x: '2021-11-07 01:00:28',
								y: 13
							},
							{
								x: '2021-11-07 09:00:28',
								y: 45
							}
						],
						backgroundColor: 'orange'
					}
				]
			},
			card: true
		},
		{
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: { foo: 42 }
			},
			configuration: {},
			card: false
		}
	]
}

const componentSets = [buttons, inputs, display]

export { componentSets }

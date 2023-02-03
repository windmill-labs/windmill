<script lang="ts" context="module">
	import {
		BarChart4,
		Binary,
		FormInput,
		Inspect,
		List,
		Monitor,
		PieChart,
		Table2,
		TextCursorInput,
		Type,
		Lock,
		Calendar,
		ToggleLeft,
		GripHorizontal,
		Code2,
		SlidersHorizontal,
		PlusSquare
	} from 'lucide-svelte'
	import type { Size } from 'svelte-grid'

	type BaseComponent<T extends string> = {
		type: T
	}

	/* 	How to add a new Svelte Component:
	  1. add the the type to the union below
	  2. add the component to the components record
	  3. add the comoonent to one of the components set (buttons, inputs, display)
	  4. (optionally) add the default code associated to it in DEFAULT_CODES
	*/
	export type TextComponent = BaseComponent<'textcomponent'>
	export type TextInputComponent = BaseComponent<'textinputcomponent'>
	export type PasswordInputComponent = BaseComponent<'passwordinputcomponent'>
	export type DateInputComponent = BaseComponent<'dateinputcomponent'>
	export type NumberInputComponent = BaseComponent<'numberinputcomponent'>
	export type SliderComponent = BaseComponent<'slidercomponent'>
	export type HtmlComponent = BaseComponent<'htmlcomponent'>
	export type VegaLiteComponent = BaseComponent<'vegalitecomponent'>
	export type PlotlyComponent = BaseComponent<'plotlycomponent'>
	export type TimeseriesComponent = BaseComponent<'timeseriescomponent'>
	export type ButtonComponent = BaseComponent<'buttoncomponent'> & {
		recomputeIds: string[] | undefined
	}
	export type FormComponent = BaseComponent<'formcomponent'> & {
		recomputeIds: string[] | undefined
	}
	export type FormButtonComponent = BaseComponent<'formbuttoncomponent'> & {
		recomputeIds: string[] | undefined
	}
	export type RunFormComponent = BaseComponent<'runformcomponent'>
	export type BarChartComponent = BaseComponent<'barchartcomponent'>
	export type PieChartComponent = BaseComponent<'piechartcomponent'>
	export type ScatterChartComponent = BaseComponent<'scatterchartcomponent'>
	export type TableComponent = BaseComponent<'tablecomponent'> & {
		actionButtons: (BaseAppComponent & ButtonComponent)[]
	}
	export type DisplayComponent = BaseComponent<'displaycomponent'>
	export type ImageComponent = BaseComponent<'imagecomponent'>
	export type InputComponent = BaseComponent<'inputcomponent'>
	export type SelectComponent = BaseComponent<'selectcomponent'>
	export type CheckboxComponent = BaseComponent<'checkboxcomponent'>
	export type RadioComponent = BaseComponent<'radiocomponent'>

	export type AppComponent = BaseAppComponent &
		(
			| DisplayComponent
			| TextInputComponent
			| PasswordInputComponent
			| DateInputComponent
			| NumberInputComponent
			| SliderComponent
			| BarChartComponent
			| TimeseriesComponent
			| HtmlComponent
			| TableComponent
			| TextComponent
			| TableComponent
			| ButtonComponent
			| PieChartComponent
			| ScatterChartComponent
			| SelectComponent
			| CheckboxComponent
			| FormComponent
			| FormButtonComponent
			| VegaLiteComponent
			| PlotlyComponent
		)

	// Dimensions key formula: <mobile width>:<mobile height>-<desktop width>:<desktop height>
	export const components: Record<
		AppComponent['type'],
		{ name: string; icon: any; dims: `${number}:${number}-${number}:${number}`; data: AppComponent }
	> = {
		displaycomponent: {
			name: 'Result',
			icon: Monitor,
			dims: '2:8-6:8',
			data: {
				id: '',
				type: 'displaycomponent',
				componentInput: {
					type: 'static',
					fieldType: 'object',
					value: { foo: 42 }
				},
				configuration: {},
				card: false
			}
		},
		textcomponent: {
			name: 'Text',
			icon: Type,
			dims: '1:1-3:1',
			data: {
				softWrap: true,
				horizontalAlignment: 'left',
				verticalAlignment: 'top',
				id: '',
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
			}
		},
		buttoncomponent: {
			name: 'Button',
			icon: Inspect,
			dims: '1:1-2:1',
			data: {
				...defaultAlignement,
				softWrap: true,
				id: '',
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
			}
		},
		formcomponent: {
			name: 'Form',
			icon: FormInput,
			dims: '3:5-6:5',
			data: {
				horizontalAlignment: 'center',
				id: '',
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
			}
		},
		formbuttoncomponent: {
			name: 'Modal Form',
			icon: PlusSquare,
			dims: '2:8-6:8',
			data: {
				horizontalAlignment: 'center',
				verticalAlignment: 'center',
				id: '',
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
		},
		piechartcomponent: {
			name: 'Pie Chart',
			icon: PieChart,
			dims: '2:8-6:8',
			data: {
				id: '',
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
			}
		},
		barchartcomponent: {
			name: 'Bar/Line Chart',
			icon: BarChart4,
			dims: '2:8-6:8',
			data: {
				id: '',
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
			}
		},
		htmlcomponent: {
			name: 'HTML',
			icon: Code2,
			dims: '1:2-1:2',
			data: {
				softWrap: false,
				id: '',
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
			}
		},
		vegalitecomponent: {
			name: 'Vega Lite',
			icon: PieChart,
			dims: '2:8-6:8',
			data: {
				softWrap: false,
				id: '',
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
			}
		},
		plotlycomponent: {
			name: 'Plotly',
			icon: PieChart,
			dims: '2:8-6:8',
			data: {
				softWrap: false,
				id: '',
				type: 'plotlycomponent',
				componentInput: {
					type: 'static',
					fieldType: 'object',
					value: {
						type: 'bar',
						x: [1, 2, 3, 4],
						y: [5, 10, 2, 8],
						marker: {
							color: '#C8A2C8',
							line: {
								width: 2.5
							}
						}
					}
				},
				configuration: {},
				card: false
			}
		},
		timeseriescomponent: {
			name: 'Timeseries',
			icon: GripHorizontal,
			dims: '2:8-6:8',
			data: {
				id: '',
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
			}
		},
		scatterchartcomponent: {
			name: 'Scatter Chart',
			icon: GripHorizontal,
			dims: '2:8-6:8',
			data: {
				id: '',
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
			}
		},
		tablecomponent: {
			name: 'Table',
			icon: Table2,
			dims: '3:10-6:10',
			data: {
				id: '',
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
			}
		},
		checkboxcomponent: {
			name: 'Toggle',
			icon: ToggleLeft,
			dims: '1:1-2:1',
			data: {
				...defaultAlignement,
				softWrap: true,
				id: '',
				type: 'checkboxcomponent',
				componentInput: undefined,
				configuration: {
					label: {
						type: 'static',
						value: 'Label',
						fieldType: 'text'
					},
					defaultValue: {
						type: 'static',
						value: undefined,
						fieldType: 'boolean'
					}
				},
				card: false
			}
		},
		textinputcomponent: {
			name: 'Text Input',
			icon: TextCursorInput,
			dims: '2:1-2:1',
			data: {
				softWrap: true,
				verticalAlignment: 'center',
				id: '',
				type: 'textinputcomponent',
				componentInput: undefined,
				configuration: {
					placeholder: {
						type: 'static',
						value: 'Type...',
						fieldType: 'text',
						onlyStatic: true
					},
					defaultValue: {
						type: 'static',
						value: undefined,
						fieldType: 'text'
					}
				},
				card: false
			}
		},
		selectcomponent: {
			name: 'Select',
			icon: List,
			dims: '2:1-3:1',
			data: {
				verticalAlignment: 'center',
				id: '',
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
		},
		numberinputcomponent: {
			name: 'Number',
			icon: Binary,
			dims: '2:1-3:1',
			data: {
				softWrap: true,
				verticalAlignment: 'center',
				id: '',
				type: 'numberinputcomponent',
				componentInput: undefined,
				configuration: {
					placeholder: {
						type: 'static',
						value: 'Type...',
						fieldType: 'text',
						onlyStatic: true
					},
					defaultValue: {
						type: 'static',
						value: undefined,
						fieldType: 'number'
					}
				},
				card: false
			}
		},
		slidercomponent: {
			name: 'Slider',
			icon: SlidersHorizontal,
			dims: '2:1-2:1',
			data: {
				softWrap: true,
				verticalAlignment: 'center',
				id: '',
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
			}
		},
		passwordinputcomponent: {
			name: 'Password',
			icon: Lock,
			dims: '2:1-3:1',
			data: {
				softWrap: true,
				verticalAlignment: 'center',
				id: '',
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
			}
		},
		dateinputcomponent: {
			name: 'Date Input',
			icon: Calendar,
			dims: '2:1-3:1',
			data: {
				softWrap: true,
				verticalAlignment: 'center',
				id: '',
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
					},
					defaultValue: {
						type: 'static',
						value: undefined,
						fieldType: 'date'
					}
				},
				card: false
			}
		}
	}

	const inputs: ComponentSet = {
		title: 'Inputs',
		components: [
			'textinputcomponent',
			'passwordinputcomponent',
			'numberinputcomponent',
			'slidercomponent',
			'dateinputcomponent',
			'checkboxcomponent',
			'selectcomponent'
		]
	}

	const buttons: ComponentSet = {
		title: 'Buttons',
		components: ['buttoncomponent', 'formcomponent', 'formbuttoncomponent']
	}

	const display: ComponentSet = {
		title: 'Display',
		components: [
			'htmlcomponent',
			'vegalitecomponent',
			'plotlycomponent',
			'textcomponent',
			'tablecomponent',
			'piechartcomponent',
			'barchartcomponent',
			'scatterchartcomponent',
			'timeseriescomponent',
			'displaycomponent'
		]
	}

	const componentSets = [buttons, inputs, display]

	const DEFAULT_CODES: Partial<Record<AppComponent['type'], Record<'deno' | 'python3', string>>> = {
		tablecomponent: {
			deno: `export async function main() {
	return [
		{
			"id": 1,
			"name": "A cell with a long name",
			"age": 42
		},
		{
			"id": 2,
			"name": "A briefer cell",
			"age": 84
		}
	]
}`,
			python3: `def main():
	return [
		{
			"id": 1,
			"name": "A cell with a long name",
			"age": 42
		},
		{
			"id": 2,
			"name": "A briefer cell",
			"age": 84
		}
	]`
		},
		textcomponent: {
			deno: `export async function main() {
	return "foo"
}`,
			python3: `def main():
	return "foo"`
		},
		barchartcomponent: {
			deno: `export async function main() {
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Bar",
			"Charts",
			"<3"
		]
	}
}`,
			python3: `def main():
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Bar",
			"Charts",
			"<3"
		]
	}`
		},
		displaycomponent: {
			deno: `export async function main() {
	return {
		"foo": 42
	}
}`,
			python3: `def main():
	return {
		"foo": 42
	}`
		},
		htmlcomponent: {
			deno: `export async function main() {
	return \`<img
	src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
	Hello world
</h1>\`
}`,
			python3: `def main():
	return '''<img
	src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
	Hello world
</h1>'''`
		},
		vegalitecomponent: {
			deno: `export async function main() {
	return {
		data: {
			values: [
				{ a: "A", b: 28 },
				{ a: "B", b: 55 },
				{ a: "C", b: 43 },
				{ a: "D", b: 91 },
			],
		},
		mark: "bar",
		encoding: {
			x: { field: "a", type: "ordinal" },
			y: { field: "b", type: "quantitative" },
		},
	}
}`,
			python3: `def main():
	return {
		"data": {
			"values": [
				{ "a": "A", "b": 28 },
				{ "a": "B", "b": 55 },
				{ "a": "C", "b": 43 },
				{ "a": "D", "b": 91 },
			],
		},
		"mark": "bar",
		"encoding": {
			"x": { "field": "a", "type": "ordinal" },
			"y": { "field": "b", "type": "quantitative" },
		},
	}`
		},
		plotlycomponent: {
			deno: `export async function main() {
	return {
		type: 'bar',
		x: [1, 2, 3, 4],
		y: [5, 10, 2, 8],
		marker: {
			color: '#C8A2C8',
			line: {
				width: 2.5				  
			}
		}
	};
}`,
			python3: `def main():
	return {
		"type": "bar",
		"x": [1, 2, 3, 4],
		"y": [5, 10, 2, 8],
		"marker": {
			"color": "#C8A2C8",
			"line": {
				"width": 2.5				  
			}
		}
	}`
		},
		piechartcomponent: {
			deno: `export async function main() {
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Pie",
			"Charts",
			"<3"
		]
	}
}`,
			python3: `def main():
	return {
		"data": [
			25,
			50,
			25
		],
		"labels": [
			"Pie",
			"Charts",
			"<3"
		]
	}`
		},
		scatterchartcomponent: {
			deno: `export async function main() {
	return [
		{
			"label": "foo",
			"data": [
				{ "x": 25, "y": 50 },
				{ "x": 23, "y": 23 },
				{ "x": 12, "y": 37 }
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{ "x": 32, "y": 32 },
				{ "x": 25, "y": 42 },
				{ "x": 3, "y": 27 }
			],
			"backgroundColor": "orange"
		}
	]
}`,
			python3: `def main():
	return [
		{
			"label": "foo",
			"data": [
				{ "x": 25, "y": 50 },
				{ "x": 23, "y": 23 },
				{ "x": 12, "y": 37 }
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{ "x": 32, "y": 32 },
				{ "x": 25, "y": 42 },
				{ "x": 3, "y": 27 }
			],
			"backgroundColor": "orange"
		}
	]`
		},
		timeseriescomponent: {
			deno: `export async function main() {
	return [
		{
			"label": "foo",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 50
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 60
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 20
				}
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 20
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 13
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 45
				}
			],
			"backgroundColor": "orange"
		}
	]
}`,
			python3: `def main():
	return [
		{
			"label": "foo",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 50
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 60
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 20
				}
			],
			"backgroundColor": "rgb(255, 12, 137)"
		},
		{
			"label": "bar",
			"data": [
				{
					"x": "2021-11-06 23:39:30",
					"y": 20
				},
				{
					"x": "2021-11-07 01:00:28",
					"y": 13
				},
				{
					"x": "2021-11-07 09:00:28",
					"y": 45
				}
			],
			"backgroundColor": "orange"
		}
	]`
		}
	} as const

	export { componentSets }

	export function defaultCode(component: string, language: string): string | undefined {
		return DEFAULT_CODES[component]?.[language]
	}

	export function getRecommendedDimensionsByComponent(
		componentType: AppComponent['type'],
		column: number
	): Size {
		const size = components[componentType].dims.split('-')[column === 3 ? 0 : 1].split(':')
		return { w: +size[0], h: +size[1] }
	}
</script>

<script lang="ts">
	import { getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import type { AppEditorContext, BaseAppComponent, ComponentSet } from '../types'
	import { Loader2 } from 'lucide-svelte'
	import AppBarChart from '../components/dataDisplay/AppBarChart.svelte'
	import AppDisplayComponent from '../components/AppDisplayComponent.svelte'
	import AppTable from '../components/table/AppTable.svelte'
	import AppText from '../components/dataDisplay/AppText.svelte'
	import AppButton from '../components/buttons/AppButton.svelte'
	import AppPieChart from '../components/dataDisplay/AppPieChart.svelte'
	import AppSelect from '../components/selectInputs/AppSelect.svelte'
	import AppCheckbox from '../components/selectInputs/AppCheckbox.svelte'
	import AppTextInput from '../components/textInputs/AppTextInput.svelte'
	import AppNumberInput from '../components/numberInputs/AppNumberInput.svelte'
	import AppDateInput from '../components/dateInputs/AppDateInput.svelte'
	import ComponentHeader from './ComponentHeader.svelte'
	import AppForm from '../components/form/AppForm.svelte'
	import AppScatterChart from '../components/dataDisplay/AppScatterChart.svelte'
	import AppTimeseries from '../components/dataDisplay/AppTimeseries.svelte'
	import AppHtml from '../components/dataDisplay/AppHtml.svelte'
	import AppSliderInputs from '../components/numberInputs/AppSliderInputs.svelte'
	import AppFormButton from '../components/form/AppFormButton.svelte'
	import VegaLiteHtml from '../components/dataDisplay/VegaLiteHtml.svelte'
	import PlotlyHtml from '../components/dataDisplay/PlotlyHtml.svelte'
	import { defaultAlignement } from './componentsPanel/componentDefaultProps'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false

	const { staticOutputs, mode, connectingInput } = getContext<AppEditorContext>('AppEditorContext')
	let hover = false
	let initializing: boolean | undefined = undefined
</script>

<div
	on:pointerenter={() => (hover = true)}
	on:pointerleave={() => (hover = false)}
	class="h-full flex flex-col w-full component"
>
	{#if $mode !== 'preview'}
		<ComponentHeader {hover} {pointerdown} {component} {selected} on:delete on:lock {locked} />
	{/if}

	<div
		on:pointerdown={(e) => {
			// Removed in https://github.com/windmill-labs/windmill/pull/1171
			// In case of a bug, try stopping propagation on the native event
			// and dispatch a custom event: `e?.stopPropagation(); dispatch('select');`
			// if ($mode === 'preview') {
			// 	e?.stopPropagation()
			// }
		}}
		class={classNames(
			'border h-full bg-white',
			selected && $mode !== 'preview' ? 'border-blue-500' : 'border-white',
			!selected && $mode !== 'preview' && !component.card ? 'border-gray-100' : '',
			$mode !== 'preview' && !$connectingInput.opened ? 'hover:border-blue-500' : '',
			component.softWrap ? '' : 'overflow-auto',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto'
		)}
	>
		{#if component.type === 'displaycomponent'}
			<AppDisplayComponent
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'barchartcomponent'}
			<AppBarChart
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'timeseriescomponent'}
			<AppTimeseries
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'htmlcomponent'}
			<AppHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'vegalitecomponent'}
			<VegaLiteHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'plotlycomponent'}
			<PlotlyHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'scatterchartcomponent'}
			<AppScatterChart
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'piechartcomponent'}
			<AppPieChart
				{...component}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
			/>
		{:else if component.type === 'tablecomponent'}
			<AppTable
				{...component}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
				bind:actionButtons={component.actionButtons}
			/>
		{:else if component.type === 'textcomponent'}
			<AppText
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'buttoncomponent'}
			<AppButton
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'selectcomponent'}
			<AppSelect {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'formcomponent'}
			<AppForm
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'formbuttoncomponent'}
			<AppFormButton
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'checkboxcomponent'}
			<AppCheckbox {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'textinputcomponent'}
			<AppTextInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'passwordinputcomponent'}
			<AppTextInput
				inputType="password"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<AppDateInput
				inputType="date"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'numberinputcomponent'}
			<AppNumberInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'slidercomponent'}
			<AppSliderInputs {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{/if}
	</div>
</div>
{#if initializing}
	<div
		out:fade={{ duration: 200 }}
		class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
	>
		<Loader2 class="animate-spin" size={16} />
		<span class="text-xs mt-1">Loading</span>
	</div>
{/if}

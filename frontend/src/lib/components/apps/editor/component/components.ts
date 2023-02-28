import type { Size } from '@windmill-labs/svelte-grid'
import type { IntRange } from '../../../../common'
import type { NARROW_GRID_COLUMNS, WIDE_GRID_COLUMNS } from '../../gridUtils'
import { defaultAlignement } from '../componentsPanel/componentDefaultProps'
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
	PlusSquare,
	ListOrdered,
	BoxSelect,
	Smile,
	DollarSign,
	SeparatorHorizontal,
	SeparatorVertical,
	Paperclip,
	Image
} from 'lucide-svelte'
import type { BaseAppComponent } from '../../types'

type BaseComponent<T extends string> = {
	type: T
}

export type TextComponent = BaseComponent<'textcomponent'>
export type TextInputComponent = BaseComponent<'textinputcomponent'>
export type PasswordInputComponent = BaseComponent<'passwordinputcomponent'>
export type DateInputComponent = BaseComponent<'dateinputcomponent'>
export type NumberInputComponent = BaseComponent<'numberinputcomponent'>
export type CurrencyComponent = BaseComponent<'currencycomponent'>
export type SliderComponent = BaseComponent<'slidercomponent'>
export type RangeComponent = BaseComponent<'rangecomponent'>
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
export type AggridComponent = BaseComponent<'aggridcomponent'>
export type DisplayComponent = BaseComponent<'displaycomponent'>
export type ImageComponent = BaseComponent<'imagecomponent'>
export type InputComponent = BaseComponent<'inputcomponent'>
export type SelectComponent = BaseComponent<'selectcomponent'>
export type CheckboxComponent = BaseComponent<'checkboxcomponent'>
export type RadioComponent = BaseComponent<'radiocomponent'>
export type IconComponent = BaseComponent<'iconcomponent'>
export type HorizontalDividerComponent = BaseComponent<'horizontaldividercomponent'>
export type VerticalDividerComponent = BaseComponent<'verticaldividercomponent'>
export type FileInputComponent = BaseComponent<'fileinputcomponent'>
export type TabsComponent = BaseComponent<'tabscomponent'> & {
	tabs: string[]
}
export type ContainerComponent = BaseComponent<'containercomponent'>

export type AppComponent = BaseAppComponent &
	(
		| DisplayComponent
		| TextInputComponent
		| PasswordInputComponent
		| DateInputComponent
		| NumberInputComponent
		| CurrencyComponent
		| SliderComponent
		| RangeComponent
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
		| TabsComponent
		| ContainerComponent
		| IconComponent
		| HorizontalDividerComponent
		| VerticalDividerComponent
		| FileInputComponent
		| ImageComponent
		| AggridComponent
	)

export type AppComponentDimensions = `${IntRange<
	1,
	typeof NARROW_GRID_COLUMNS
>}:${number}-${IntRange<1, typeof WIDE_GRID_COLUMNS>}:${number}`

export type AppComponentConfig = {
	name: string
	icon: any
	/**
	 * Dimensions key formula:
	 * [**mobile width**]:[**mobile height**]-[**desktop width**]:[**desktop height**]
	 */
	dims: AppComponentDimensions
	data: AppComponent
}

export function getRecommendedDimensionsByComponent(
	componentType: AppComponent['type'],
	column: number
): Size {
	const size = components[componentType].dims.split('-')[column === 3 ? 0 : 1].split(':')
	return { w: +size[0], h: +size[1] }
}

export const components: Record<AppComponent['type'], AppComponentConfig> = {
	displaycomponent: {
		name: 'Rich Result',
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
			customCss: {
				header: { class: '', style: '' },
				container: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	containercomponent: {
		name: 'Container',
		icon: BoxSelect,
		dims: '2:8-6:8',
		data: {
			softWrap: true,
			id: '',
			type: 'containercomponent',
			configuration: {
				noPadding: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true
				}
			},
			customCss: {
				container: { class: '', style: '' }
			} as const,
			componentInput: undefined,
			card: false,
			numberOfSubgrids: 1
		}
	},
	textcomponent: {
		name: 'Text',
		icon: Type,
		dims: '1:1-3:1',
		data: {
			softWrap: false,
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
				copyButton: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					onlyStatic: true
				}
			},
			customCss: {
				text: { class: '', style: '' }
			} as const,
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
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',
					onlyStatic: true,
					value: false
				},
				disabled: {
					fieldType: 'boolean',
					type: 'eval',
					expr: 'false'
				},
				goto: {
					tooltip: 'Go to an url on success if not empty',
					fieldType: 'text',
					type: 'static',
					value: ''
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select',
					onlyStatic: true
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select',
					onlyStatic: true
				},
				triggerOnAppLoad: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					onlyStatic: true
				}
			},
			customCss: {
				button: { style: '', class: '' }
			} as const,
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
				},
				goto: {
					tooltip: 'Go to an url on success if not empty',
					fieldType: 'text',
					type: 'static',
					value: ''
				}
			},
			customCss: {
				container: { class: '', style: '' },
				button: { class: '', style: '' }
			} as const,
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
			customCss: {
				button: { class: '', style: '' },
				popup: { class: '', style: '' }
			} as const,
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
			customCss: {
				container: { class: '', style: '' }
			} as const,
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
			customCss: {
				container: { class: '', style: '' }
			} as const,
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
			customCss: {
				container: { class: '', style: '' }
			} as const,
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
			customCss: {},
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
			customCss: {},
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
			customCss: {
				container: { class: '', style: '' }
			} as const,
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
			customCss: {
				container: { class: '', style: '' }
			} as const,
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
			customCss: {
				container: { class: '', style: '' },
				tableHeader: { class: '', style: '' },
				tableBody: { class: '', style: '' },
				tableFooter: { class: '', style: '' }
			} as const,
			card: true,
			actionButtons: []
		}
	},
	aggridcomponent: {
		name: 'AgGrid Table',
		icon: Table2,
		dims: '3:10-6:10',
		data: {
			id: '',
			type: 'aggridcomponent',
			configuration: {
				columnDefs: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'object',
					value: [{ field: 'id' }, { field: 'name', editable: true }, { field: 'age' }]
				},
				allEditable: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true
				},
				pagination: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true
				},
				pageSize: {
					type: 'static',
					fieldType: 'number',
					value: 10,
					onlyStatic: true
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
			customCss: {},
			card: true
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
			customCss: {
				text: { class: '', style: '' }
			} as const,
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
			customCss: {
				input: { class: '', style: '' }
			} as const,
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
				multiple: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true
				},
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select an item',
					onlyStatic: true
				}
			},
			customCss: {
				input: { style: '' }
			} as const,
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
				},
				min: {
					type: 'static',
					value: undefined,
					fieldType: 'number'
				},
				max: {
					type: 'static',
					value: undefined,
					fieldType: 'number'
				},
				step: {
					type: 'static',
					value: 1,
					fieldType: 'number'
				}
			},
			customCss: {
				input: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	currencycomponent: {
		name: 'Currency',
		icon: DollarSign,
		dims: '2:1-3:1',
		data: {
			softWrap: true,
			verticalAlignment: 'center',
			id: '',
			type: 'currencycomponent',
			componentInput: undefined,
			configuration: {
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'number'
				},
				isNegativeAllowed: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					onlyStatic: true
				},
				currency: {
					type: 'static',
					value: 'USD',
					fieldType: 'select',
					onlyStatic: true,
					optionValuesKey: 'currencyOptions'
				},
				locale: {
					type: 'static',
					value: 'en-US',
					fieldType: 'select',
					onlyStatic: true,
					optionValuesKey: 'localeOptions'
				}
			},
			customCss: {
				input: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	slidercomponent: {
		name: 'Slider',
		icon: SlidersHorizontal,
		dims: '3:1-4:1',
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
					fieldType: 'number'
				},
				defaultValue: {
					type: 'static',
					value: 20,
					fieldType: 'number'
				},
				step: {
					type: 'static',
					value: 1,
					fieldType: 'number'
				}
			},
			customCss: {
				handle: { style: '' },
				limits: { class: '', style: '' },
				value: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	rangecomponent: {
		name: 'Range',
		icon: SlidersHorizontal,
		dims: '3:2-4:2',
		data: {
			softWrap: true,
			verticalAlignment: 'center',
			id: '',
			type: 'rangecomponent',
			componentInput: undefined,
			configuration: {
				min: {
					type: 'static',
					value: 0,
					fieldType: 'number'
				},
				max: {
					type: 'static',
					value: 42,
					fieldType: 'number'
				},
				defaultLow: {
					type: 'static',
					value: 10,
					fieldType: 'number'
				},
				defaultHigh: {
					type: 'static',
					value: 20,
					fieldType: 'number'
				},
				step: {
					type: 'static',
					value: 1,
					fieldType: 'number'
				}
			},
			customCss: {
				handles: { style: '' },
				limits: { class: '', style: '' },
				values: { class: '', style: '' }
			} as const,
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
			customCss: {
				input: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	dateinputcomponent: {
		name: 'Date',
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
			customCss: {
				input: { class: '', style: '' }
			} as const,
			card: false
		}
	},
	tabscomponent: {
		name: 'Tabs',
		icon: ListOrdered,
		dims: '2:8-6:8',
		data: {
			softWrap: true,
			id: '',
			type: 'tabscomponent',
			configuration: {
				noPadding: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true
				}
			},
			componentInput: undefined,
			card: false,
			numberOfSubgrids: 2,
			tabs: ['First tab', 'Second tab']
		}
	},
	iconcomponent: {
		name: 'Icon',
		icon: Smile,
		dims: '1:3-1:2',
		data: {
			softWrap: false,
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			id: '',
			type: 'iconcomponent',
			componentInput: undefined,
			configuration: {
				icon: {
					type: 'static',
					value: 'Smile',
					fieldType: 'icon-select'
				},
				color: {
					type: 'static',
					value: 'currentColor',
					fieldType: 'text'
				},
				size: {
					type: 'static',
					value: 24,
					fieldType: 'number',
					onlyStatic: true
				},
				strokeWidth: {
					type: 'static',
					value: 2,
					fieldType: 'number',
					onlyStatic: true
				}
			},
			customCss: {},
			card: false
		}
	},
	horizontaldividercomponent: {
		name: 'Divider X',
		icon: SeparatorHorizontal,
		dims: '3:1-12:1',
		data: {
			verticalAlignment: 'center',
			id: '',
			type: 'horizontaldividercomponent',
			componentInput: undefined,
			configuration: {
				size: {
					type: 'static',
					value: 2,
					fieldType: 'number',
					onlyStatic: true
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			customCss: {},
			card: false
		}
	},
	verticaldividercomponent: {
		name: 'Divider Y',
		icon: SeparatorVertical,
		dims: '1:4-1:6',
		data: {
			horizontalAlignment: 'center',
			id: '',
			type: 'verticaldividercomponent',
			componentInput: undefined,
			configuration: {
				size: {
					type: 'static',
					value: 2,
					fieldType: 'number',
					onlyStatic: true
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			customCss: {},
			card: false
		}
	},
	fileinputcomponent: {
		name: 'File Input',
		icon: Paperclip,
		dims: '3:4-6:4',
		data: {
			id: '',
			type: 'fileinputcomponent',
			componentInput: undefined,
			configuration: {
				acceptedFileTypes: {
					type: 'static',
					value: ['image/*', 'application/pdf'],
					fieldType: 'array',
					onlyStatic: true
				},
				allowMultiple: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					tooltip: 'If allowed, the user will be able to select more than one file',
					onlyStatic: true
				},
				text: {
					type: 'static',
					value: 'Drag and drop files or click to select them',
					fieldType: 'text',
					onlyStatic: true
				}
			},
			customCss: {},
			card: false
		}
	},
	imagecomponent: {
		name: 'Image',
		icon: Image,
		dims: '3:4-5:4',
		data: {
			id: '',
			type: 'imagecomponent',
			componentInput: undefined,
			configuration: {
				source: {
					type: 'static',
					value: '/logo.svg',
					fieldType: 'text',
					fileUpload: {
						accept: 'image/*',
						base64: true
					}
				},
				imageFit: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					optionValuesKey: 'objectFitOptions',
					value: 'contain'
				},
				altText: {
					type: 'static',
					value: '',
					fieldType: 'text',
					onlyStatic: true,
					tooltip: "This text will appear if the image can't be loaded for any reason"
				},
				customStyles: {
					type: 'static',
					value: '',
					fieldType: 'textarea',
					onlyStatic: true
				}
			},
			customCss: {},
			card: false
		}
	}
}

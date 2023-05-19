import type { IntRange } from '../../../../common'
import type { NARROW_GRID_COLUMNS, WIDE_GRID_COLUMNS } from '../../gridUtils'
import { BUTTON_COLORS } from '../../../common'

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
	Image,
	SidebarClose,
	MapPin,
	FlipHorizontal,
	FlipVertical,
	FileText,
	AtSignIcon,
	Split,
	Download,
	PanelLeft,
	PanelTopInactive
} from 'lucide-svelte'
import type {
	Aligned,
	BaseAppComponent,
	ComponentCustomCSS,
	GridItem,
	RichConfiguration,
	StaticRichConfigurations
} from '../../types'
import type { Size } from '../../svelte-grid/types'

import type { AppInputSpec, ResultAppInput, StaticAppInput } from '../../inputType'

export type BaseComponent<T extends string> = {
	type: T
}

export type RecomputeOthersSource = {
	recomputeIds: string[] | undefined
}

export type TextComponent = BaseComponent<'textcomponent'>
export type TextInputComponent = BaseComponent<'textinputcomponent'>
export type TextareaInputComponent = BaseComponent<'textareainputcomponent'>
export type PasswordInputComponent = BaseComponent<'passwordinputcomponent'>
export type EmailInputComponent = BaseComponent<'emailinputcomponent'>
export type DateInputComponent = BaseComponent<'dateinputcomponent'>
export type NumberInputComponent = BaseComponent<'numberinputcomponent'>
export type CurrencyComponent = BaseComponent<'currencycomponent'>
export type SliderComponent = BaseComponent<'slidercomponent'>
export type RangeComponent = BaseComponent<'rangecomponent'>
export type HtmlComponent = BaseComponent<'htmlcomponent'>
export type VegaLiteComponent = BaseComponent<'vegalitecomponent'>
export type PlotlyComponent = BaseComponent<'plotlycomponent'>
export type TimeseriesComponent = BaseComponent<'timeseriescomponent'>
export type ButtonComponent = BaseComponent<'buttoncomponent'> & RecomputeOthersSource
export type DownloadComponent = BaseComponent<'downloadcomponent'>
export type FormComponent = BaseComponent<'formcomponent'> & RecomputeOthersSource
export type FormButtonComponent = BaseComponent<'formbuttoncomponent'> & RecomputeOthersSource

export type RunFormComponent = BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = BaseComponent<'piechartcomponent'>
export type ScatterChartComponent = BaseComponent<'scatterchartcomponent'>
export type TableComponent = BaseComponent<'tablecomponent'> & {
	actionButtons: (BaseAppComponent & ButtonComponent & GridItem)[]
}
export type AggridComponent = BaseComponent<'aggridcomponent'>
export type DisplayComponent = BaseComponent<'displaycomponent'>
export type ImageComponent = BaseComponent<'imagecomponent'>
export type InputComponent = BaseComponent<'inputcomponent'>
export type SelectComponent = BaseComponent<'selectcomponent'> & RecomputeOthersSource
export type ResourceSelectComponent = BaseComponent<'resourceselectcomponent'> &
	RecomputeOthersSource
export type MultiSelectComponent = BaseComponent<'multiselectcomponent'>
export type CheckboxComponent = BaseComponent<'checkboxcomponent'> & RecomputeOthersSource
export type RadioComponent = BaseComponent<'radiocomponent'>
export type IconComponent = BaseComponent<'iconcomponent'>
export type HorizontalDividerComponent = BaseComponent<'horizontaldividercomponent'>
export type VerticalDividerComponent = BaseComponent<'verticaldividercomponent'>
export type FileInputComponent = BaseComponent<'fileinputcomponent'>
export type TabsComponent = BaseComponent<'tabscomponent'> & {
	tabs: string[]
}
export type ContainerComponent = BaseComponent<'containercomponent'>
export type DrawerComponent = BaseComponent<'drawercomponent'>
export type MapComponent = BaseComponent<'mapcomponent'>
export type VerticalSplitPanesComponent = BaseComponent<'verticalsplitpanescomponent'> & {
	panes: number[]
}
export type HorizontalSplitPanesComponent = BaseComponent<'horizontalsplitpanescomponent'> & {
	panes: number[]
}
export type PdfComponent = BaseComponent<'pdfcomponent'>
export type ModalComponent = BaseComponent<'modalcomponent'>
export type StepperComponent = BaseComponent<'steppercomponent'> & {
	tabs: string[]
}
export type ConditionalWrapperComponent = BaseComponent<'conditionalwrapper'> & {
	conditions: RichConfiguration[]
}

export type Schemaformcomponent = BaseComponent<'schemaformcomponent'>
export type SelectTabComponent = BaseComponent<'selecttabcomponent'>
export type SelectStepComponent = BaseComponent<'selectstepcomponent'>

export type TypedComponent =
	| DisplayComponent
	| TextInputComponent
	| TextareaInputComponent
	| PasswordInputComponent
	| EmailInputComponent
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
	| ButtonComponent
	| PieChartComponent
	| ScatterChartComponent
	| SelectComponent
	| ResourceSelectComponent
	| MultiSelectComponent
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
	| DrawerComponent
	| MapComponent
	| VerticalSplitPanesComponent
	| HorizontalSplitPanesComponent
	| PdfComponent
	| ModalComponent
	| StepperComponent
	| Schemaformcomponent
	| SelectTabComponent
	| ConditionalWrapperComponent
	| SelectStepComponent
	| DownloadComponent

export type AppComponent = BaseAppComponent & TypedComponent

export type AppComponentDimensions = `${IntRange<
	1,
	typeof NARROW_GRID_COLUMNS
>}:${number}-${IntRange<1, typeof WIDE_GRID_COLUMNS>}:${number}`

export function getRecommendedDimensionsByComponent(
	componentType: AppComponent['type'],
	column: number
): Size {
	const size = components[componentType].dims.split('-')[column === 3 ? 0 : 1].split(':')
	return { w: +size[0], h: +size[1] }
}

export type AppComponentConfig<T extends TypedComponent['type']> = {
	name: string
	icon: any
	/**
	 * Dimensions key formula:
	 * [**mobile width**]:[**mobile height**]-[**desktop width**]:[**desktop height**]
	 */
	dims: AppComponentDimensions
	/**
	 * If `true` then the wrapper will allow items to flow outside of it's borders.
	 *
	 * *For example when the component has a popup like `Select`*
	 */
	initialData: InitialAppComponent
	customCss: ComponentCustomCSS<T>
}

export type PresetComponentConfig = {
	name: string
	icon: any
	targetComponent: keyof typeof components
	configuration: object
	type: string
}

export interface InitialAppComponent extends Partial<Aligned> {
	componentInput?: StaticAppInput | ResultAppInput | undefined
	configuration: StaticRichConfigurations
	// Number of subgrids
	numberOfSubgrids?: number
	recomputeIds?: boolean
	actionButtons?: boolean
	tabs?: string[]
	panes?: number[]
	conditions?: AppInputSpec<'boolean', boolean>[]
}

const buttonColorOptions = [...BUTTON_COLORS]

export const selectOptions = {
	buttonColorOptions,
	tabsKindOptions: ['tabs', 'sidebar', 'invisibleOnView'],
	buttonSizeOptions: ['xs', 'sm', 'md', 'lg', 'xl'],
	tableSearchOptions: ['By Component', 'By Runnable', 'Disabled'],
	chartThemeOptions: ['theme1', 'theme2', 'theme3'],
	textStyleOptions: ['Title', 'Subtitle', 'Body', 'Label', 'Caption'],
	currencyOptions: ['USD', 'EUR', 'GBP', 'CAD', 'AUD', 'JPY', 'CNY', 'INR', 'BRL'],
	localeOptions: [
		'en-US',
		'en-GB',
		'en-IE',
		'de-DE',
		'fr-FR',
		'br-FR',
		'ja-JP',
		'pt-TL',
		'fr-CA',
		'en-CA'
	],
	objectFitOptions: ['contain', 'cover', 'fill'],
	splitPanelOptions: ['2', '3', '4'],
	formorientationOptions: ['Horizontal', 'Vertical']
}

const labels = {
	none: 'Do nothing',
	errorOverlay: 'Show error overlay',
	gotoUrl: 'Go to an url',
	setTab: 'Set the tab of a tabs component',
	sendToast: 'Display a toast notification',
	sendErrorToast: 'Display an error toast notification'
}

const onSuccessClick = {
	type: 'oneOf',
	tooltip: 'Action to perform on success',
	selected: 'none',
	labels,
	configuration: {
		none: {},
		gotoUrl: {
			url: {
				tooltip: 'Go to the given url, absolute or relative',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: '/apps/get/foo'
			},
			newTab: {
				tooltip: 'Open the url in a new tab',
				fieldType: 'boolean',
				type: 'static',
				value: true
			}
		},
		setTab: {
			setTab: {
				type: 'static',
				value: [] as Array<{ id: string; index: number }>,
				fieldType: 'array',
				subFieldType: 'tab-select',
				tooltip: 'Set the tabs id and index to go to on success'
			}
		},
		sendToast: {
			message: {
				tooltip: 'The message of the toast to diplay',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: 'Hello there'
			}
		}
	}
} as const

const onErrorClick = {
	type: 'oneOf',
	tooltip: 'Action to perform on error',
	selected: 'errorOverlay',
	labels,
	configuration: {
		errorOverlay: {},
		gotoUrl: {
			url: {
				tooltip: 'Go to the given url, absolute or relative',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: '/apps/get/foo'
			},
			newTab: {
				tooltip: 'Open the url in a new tab',
				fieldType: 'boolean',
				type: 'static',
				value: true
			}
		},
		setTab: {
			setTab: {
				type: 'static',
				value: [] as Array<{ id: string; index: number }>,
				fieldType: 'array',
				subFieldType: 'tab-select',
				tooltip: 'Set the tabs id and index to go to on error'
			}
		},
		sendErrorToast: {
			message: {
				tooltip: 'The message of the toast to diplay',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: 'Hello there'
			},
			appendError: {
				tooltip: 'Append the error message to the toast',
				fieldType: 'boolean',
				type: 'static',
				value: true
			}
		}
	}
} as const

const paginationOneOf = {
	type: 'oneOf',
	selected: 'auto',
	labels: {
		auto: 'Auto',
		manual: 'Manual'
	},
	configuration: {
		auto: {
			pageSize: {
				type: 'static',
				fieldType: 'number',
				value: 20,
				onlyStatic: true,
				tooltip: 'Number of rows per page'
			}
		},
		manual: {
			pageCount: {
				type: 'static',
				fieldType: 'number',
				value: -1,
				tooltip: 'Number of pages (-1 if you do not know)'
			}
		}
	}
} as const

export const components = {
	displaycomponent: {
		name: 'Rich Result',
		icon: Monitor,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			header: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: { foo: 42 }
			},
			configuration: {}
		}
	},
	containercomponent: {
		name: 'Container',
		icon: BoxSelect,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: undefined,
			numberOfSubgrids: 1
		}
	},
	textcomponent: {
		name: 'Text',
		icon: Type,
		dims: '1:1-3:1' as AppComponentDimensions,

		customCss: {
			text: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'left',
			verticalAlignment: 'top',

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
					selectOptions: selectOptions.textStyleOptions,
					value: 'Body' as 'Title' | 'Subtitle' | 'Body' | 'Label' | 'Caption'
				},
				copyButton: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					onlyStatic: true
				},
				tooltip: {
					type: 'static',
					value: '',
					fieldType: 'text',
					onlyStatic: true,
					tooltip: 'Tooltip text if not empty'
				}
			}
		}
	},
	buttoncomponent: {
		name: 'Button',
		icon: Inspect,
		dims: '1:1-2:1' as AppComponentDimensions,

		customCss: {
			button: { style: '', class: '' },
			container: { style: '', class: '' }
		},
		initialData: {
			...defaultAlignement,
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			},
			recomputeIds: true,
			configuration: {
				label: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					selectOptions: selectOptions.buttonColorOptions,
					value: 'blue',
					tooltip: 'Theses presets can be overwritten with custom styles.'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',
					onlyStatic: true,
					value: false,
					tooltip:
						'This will make the button fill the container width and height. Height and width can be overwritten with custom styles.'
				},
				disabled: {
					fieldType: 'boolean',
					type: 'static',
					value: false
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
				},
				onSuccess: onSuccessClick,
				onError: onErrorClick
			}
		}
	},
	downloadcomponent: {
		name: 'Download Button',
		icon: Download,
		dims: '1:1-2:1' as AppComponentDimensions,

		customCss: {
			button: { style: '', class: '' }
		},
		initialData: {
			...defaultAlignement,
			configuration: {
				source: {
					type: 'static',
					value: '',
					fieldType: 'text',
					fileUpload: {
						accept: '*',
						convertTo: 'base64'
					},
					placeholder: 'Enter URL or upload file'
				},
				filename: {
					type: 'static',
					fieldType: 'text',
					value: 'windmill.file'
				},
				label: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					selectOptions: selectOptions.buttonColorOptions,
					value: 'blue'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',
					onlyStatic: true,
					value: false
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
				}
			}
		}
	},
	formcomponent: {
		name: 'Submit form',
		icon: FormInput,
		dims: '3:5-6:5' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			button: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			},
			recomputeIds: true,
			configuration: {
				label: {
					type: 'static',
					value: 'Submit',
					fieldType: 'text'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					value: 'dark',
					selectOptions: selectOptions.buttonColorOptions
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions
				},
				onSuccess: onSuccessClick,
				onError: onErrorClick
			}
		}
	},
	formbuttoncomponent: {
		name: 'Modal Form',
		icon: PlusSquare,
		dims: '1:1-2:1' as AppComponentDimensions,
		customCss: {
			button: { class: '', style: '' },
			popup: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			},
			recomputeIds: true,
			configuration: {
				label: {
					type: 'static',
					value: 'Open popup',
					fieldType: 'text'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					value: 'dark',
					selectOptions: buttonColorOptions,
					tooltip: 'Theses presets can be overwritten with custom styles.'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions
				},
				onSuccess: onSuccessClick,
				onError: onErrorClick,
				disabled: {
					fieldType: 'boolean',
					type: 'static',
					value: false
				}
			}
		}
	},
	piechartcomponent: {
		name: 'Pie Chart',
		icon: PieChart,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				theme: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'select',
					selectOptions: selectOptions.chartThemeOptions,
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
			}
		}
	},
	barchartcomponent: {
		name: 'Bar/Line Chart',
		icon: BarChart4,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				theme: {
					type: 'static',
					onlyStatic: true,
					fieldType: 'select',
					selectOptions: selectOptions.chartThemeOptions,
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
			}
		}
	},
	htmlcomponent: {
		name: 'HTML',
		icon: Code2,
		dims: '1:2-1:2' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
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
			configuration: {}
		}
	},
	vegalitecomponent: {
		name: 'Vega Lite',
		icon: PieChart,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {},
		initialData: {
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
					tooltip: 'Use the canvas renderer instead of the svg one for more interactive plots'
				}
			}
		}
	},
	plotlycomponent: {
		name: 'Plotly',
		icon: PieChart,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {},
		initialData: {
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
			configuration: {
				layout: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip:
						'Layout options for the plot. See https://plotly.com/javascript/reference/layout/ for more information',
					onlyStatic: true
				}
			}
		}
	},
	timeseriescomponent: {
		name: 'Timeseries',
		icon: GripHorizontal,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
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
			} as StaticAppInput
		}
	},
	scatterchartcomponent: {
		name: 'Scatter Chart',
		icon: GripHorizontal,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
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
			} as StaticAppInput
		}
	},
	tablecomponent: {
		name: 'Table',
		icon: Table2,
		dims: '3:10-6:10' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			tableHeader: { class: '', style: '' },
			tableBody: { class: '', style: '' },
			tableFooter: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				search: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.tableSearchOptions,
					value: 'Disabled' as string,
					tooltip:
						'Search can be configured in the following ways: Disabled: The search is disabled,By Runnable: The search is done in the backend, or by component: The search is done in the frontend.'
				},
				pagination: paginationOneOf
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
			} as StaticAppInput,
			actionButtons: true
		}
	},
	aggridcomponent: {
		name: 'AgGrid Table',
		icon: Table2,
		dims: '3:10-6:10' as AppComponentDimensions,
		customCss: {},
		initialData: {
			configuration: {
				columnDefs: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'object',
					value: [{ field: 'id' }, { field: 'name', editable: true }, { field: 'age' }]
				} as StaticAppInput,
				allEditable: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true,
					tooltip: 'Configure all columns as Editable by users'
				},
				pagination: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
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
			} as StaticAppInput
		}
	},
	checkboxcomponent: {
		name: 'Toggle',
		icon: ToggleLeft,
		dims: '1:1-2:1' as AppComponentDimensions,

		customCss: {
			text: { class: '', style: '' }
		},
		initialData: {
			...defaultAlignement,
			componentInput: undefined,
			recomputeIds: true,
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
			}
		}
	},
	textinputcomponent: {
		name: 'Text Input',
		icon: TextCursorInput,
		dims: '2:1-2:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
			}
		}
	},
	textareainputcomponent: {
		name: 'Textarea',
		icon: TextCursorInput,
		dims: '2:1-2:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
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
			}
		}
	},
	selectcomponent: {
		name: 'Select',
		icon: List,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: {
				style: '',
				tooltip: 'https://github.com/rob-balfre/svelte-select/blob/master/docs/theming_variables.md'
			}
		},
		initialData: {
			recomputeIds: true,
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'labeledselect',
					value: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					]
				} as StaticAppInput,
				create: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true,
					tooltip: 'Allows user to manually add new value',
					customTitle: 'User creatable'
				},
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select an item',
					onlyStatic: true
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'object'
				}
			}
		}
	},
	multiselectcomponent: {
		name: 'Multi Select',
		icon: List,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			multiselect: {
				style: '',
				tooltip:
					'See https://multiselect.janosh.dev/#with-css-variables for the available variables'
			}
		},
		initialData: {
			componentInput: undefined,
			configuration: {
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'text',
					value: ['Foo', 'Bar']
				} as StaticAppInput,
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select items',
					onlyStatic: true
				},
				create: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					onlyStatic: true,
					tooltip: 'Allows user to manually add new value',
					customTitle: 'User creatable'
				},
				allowOverflow: {
					type: 'static',
					fieldType: 'boolean',
					value: true,
					onlyStatic: true,
					tooltip:
						'If too many items, the box overflow its container instead of having an internal scroll'
				}
			}
		}
	},
	resourceselectcomponent: {
		name: 'Resource Select',
		icon: List,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'labeledresource',
					value: []
				} as StaticAppInput,
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select an item',
					onlyStatic: true
				}
			}
		}
	},
	numberinputcomponent: {
		name: 'Number',
		icon: Binary,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
					fieldType: 'number',
					tooltip: 'Spread between each number suggestion'
				}
			}
		}
	},
	currencycomponent: {
		name: 'Currency',
		icon: DollarSign,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
					selectOptions: selectOptions.currencyOptions
				},
				locale: {
					type: 'static',
					value: 'en-US',
					fieldType: 'select',
					onlyStatic: true,
					selectOptions: selectOptions.localeOptions,
					tooltip: 'Currency format'
				}
			}
		}
	},
	slidercomponent: {
		name: 'Slider',
		icon: SlidersHorizontal,
		dims: '3:1-4:1' as AppComponentDimensions,

		customCss: {
			bar: { style: '' },
			handle: { style: '' },
			limits: { class: '', style: '' },
			value: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
					fieldType: 'number',
					tooltip: 'Spread between each number suggestion'
				}
			}
		}
	},
	rangecomponent: {
		name: 'Range',
		icon: SlidersHorizontal,
		dims: '3:2-4:2' as AppComponentDimensions,

		customCss: {
			handles: { style: '' },
			bar: { style: '' },
			limits: { class: '', style: '' },
			values: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
					fieldType: 'number',
					tooltip: 'Spread between each number suggestion'
				}
			}
		}
	},
	passwordinputcomponent: {
		name: 'Password',
		icon: Lock,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Password',
					fieldType: 'text',
					onlyStatic: true
				}
			}
		}
	},
	emailinputcomponent: {
		name: 'Email Input',
		icon: AtSignIcon,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Email',
					fieldType: 'text',
					onlyStatic: true
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'text'
				}
			}
		}
	},
	dateinputcomponent: {
		name: 'Date',
		icon: Calendar,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
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
			}
		}
	},
	tabscomponent: {
		name: 'Tabs',
		icon: ListOrdered,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			tabRow: { class: '', style: '' },
			allTabs: { class: '', style: '' },
			selectedTab: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				tabsKind: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.tabsKindOptions,
					value: 'tabs' as string,
					tooltip: `Tabs can be configured to be either horizontal (tabs), vertical (sidebar), or invisible.`
				}
			},
			componentInput: undefined,
			numberOfSubgrids: 2,
			tabs: ['First tab', 'Second tab'] as string[]
		}
	},
	steppercomponent: {
		name: 'Stepper',
		icon: ListOrdered,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			},
			numberOfSubgrids: 2,
			tabs: ['First', 'Second'] as string[]
		}
	},
	iconcomponent: {
		name: 'Icon',
		icon: Smile,
		dims: '1:3-1:2' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' },
			icon: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				icon: {
					type: 'static',
					value: 'Smile',
					fieldType: 'icon-select',
					tooltip: 'The icons can be found at https://lucide.dev/'
				},
				color: {
					type: 'static',
					value: 'currentColor',
					fieldType: 'color',
					tooltip:
						'The color of the icon can be overridden by the `background-color` property in the styling menu'
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
			}
		}
	},
	horizontaldividercomponent: {
		name: 'Divider X',
		icon: SeparatorHorizontal,
		dims: '3:1-12:1' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			divider: {
				class: '',
				style: '',
				tooltip: '`background-color` and `height` are handled by the component configuration'
			}
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				size: {
					type: 'static',
					value: 2,
					fieldType: 'number',
					onlyStatic: true,
					tooltip:
						'The height of the divider in pixels can be overridden by the `height` property in the styling menu'
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'color',
					onlyStatic: true,
					tooltip:
						'The color of the divider can be overridden by the `background-color` property in the styling menu'
				}
			}
		}
	},
	verticaldividercomponent: {
		name: 'Divider Y',
		icon: SeparatorVertical,
		dims: '1:4-1:6' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			divider: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				size: {
					type: 'static',
					value: 2,
					fieldType: 'number',
					onlyStatic: true,
					tooltip:
						'The width of the divider in pixels can be overridden by the `width` property in the styling menu'
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'color',
					onlyStatic: true,
					tooltip:
						'The color of the divider can be overridden by the `background-color` property in the styling menu'
				}
			}
		}
	},
	fileinputcomponent: {
		name: 'File Input',
		icon: Paperclip,
		dims: '3:4-6:4' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: undefined,
			configuration: {
				acceptedFileTypes: {
					type: 'static',
					value: ['image/*', 'application/pdf'] as string[],
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
			}
		}
	},
	imagecomponent: {
		name: 'Image',
		icon: Image,
		dims: '3:4-5:4' as AppComponentDimensions,
		customCss: {
			image: { class: '', style: '' }
		},
		initialData: {
			componentInput: undefined,
			configuration: {
				source: {
					type: 'static',
					value: '/logo.svg',
					fieldType: 'text',
					fileUpload: {
						accept: 'image/*',
						convertTo: 'base64'
					}
				},
				imageFit: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.objectFitOptions,
					value: 'contain',
					tooltip:
						'The image fit property can be overridden by the `object-fit` property in the styling menu'
				},
				altText: {
					type: 'static',
					value: '',
					fieldType: 'text',
					onlyStatic: true,
					tooltip: "This text will appear if the image can't be loaded for any reason"
				}
			}
		}
	},
	drawercomponent: {
		name: 'Drawer',
		icon: SidebarClose,
		dims: '1:1-2:1' as AppComponentDimensions,

		customCss: {
			button: { style: '', class: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			configuration: {
				drawerTitle: {
					type: 'static',
					fieldType: 'text',
					value: 'Drawer title',
					onlyStatic: true
				},
				label: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					selectOptions: buttonColorOptions,
					value: 'blue',
					tooltip:
						'The color of the button can be overridden by the `background-color` property in the styling menu'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',
					onlyStatic: true,
					value: false,
					tooltip:
						'This will make the button fill the container width and height. Height and width can be overwritten with custom styles.'
				},
				disabled: {
					fieldType: 'boolean',
					type: 'static',
					value: false
				}
			},
			componentInput: undefined,
			numberOfSubgrids: 1
		}
	},
	mapcomponent: {
		name: 'Map',
		icon: MapPin,
		dims: '3:6-6:10' as AppComponentDimensions,
		customCss: {
			map: { class: '', style: '' }
		},
		initialData: {
			componentInput: undefined,
			configuration: {
				longitude: {
					fieldType: 'number',
					type: 'static',
					value: 15
				},
				latitude: {
					fieldType: 'number',
					type: 'static',
					value: 50
				},
				zoom: {
					fieldType: 'number',
					type: 'static',
					value: 3
				},
				markers: {
					fieldType: 'array',
					type: 'static',
					subFieldType: 'object',
					fileUpload: {
						accept: 'application/json',
						convertTo: 'text'
					},
					value: [
						{
							lon: 12.496366,
							lat: 41.902783,
							title: 'Rome',
							radius: 7,
							color: '#dc2626',
							strokeWidth: 3,
							strokeColor: '#fca5a5'
						},
						{
							lon: -0.136439,
							lat: 51.507359,
							title: 'London'
						}
					]
				} as StaticAppInput
			}
		}
	},
	verticalsplitpanescomponent: {
		name: 'Vertical Split Panes',
		icon: FlipHorizontal,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: undefined,
			panes: [50, 50] as number[],
			numberOfSubgrids: 2
		}
	},
	horizontalsplitpanescomponent: {
		name: 'Horizontal Split Panes',
		icon: FlipVertical,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: undefined,
			panes: [50, 50] as number[],
			numberOfSubgrids: 2
		}
	},
	pdfcomponent: {
		name: 'PDF',
		icon: FileText,
		dims: '3:8-8:12' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: undefined,
			configuration: {
				source: {
					type: 'static',
					value: '/dummy.pdf',
					fieldType: 'text',
					fileUpload: {
						accept: 'application/pdf',
						convertTo: 'base64'
					},
					placeholder: 'Enter URL or upload file'
				},
				zoom: {
					fieldType: 'number',
					type: 'static',
					value: 100
				}
			}
		}
	},
	modalcomponent: {
		name: 'Modal',
		icon: SidebarClose,
		dims: '1:1-2:1' as AppComponentDimensions,
		customCss: {
			button: { class: '', style: '' },
			buttonContainer: { class: '', style: '' },
			popup: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			configuration: {
				modalTitle: {
					type: 'static',
					fieldType: 'text',
					value: 'Modal title',
					onlyStatic: true
				},
				buttonLabel: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
				},
				buttonColor: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: buttonColorOptions,
					value: 'blue',
					tooltip: 'Theses presets can be overwritten with custom styles.'
				},
				buttonSize: {
					fieldType: 'select',
					type: 'static',
					onlyStatic: true,
					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				buttonFillContainer: {
					fieldType: 'boolean',
					type: 'static',
					onlyStatic: true,
					value: false,
					tooltip:
						'This will make the button fill the container width and height. Height and width can be overwritten with custom styles.'
				},
				buttonDisabled: {
					fieldType: 'boolean',
					type: 'static',
					value: false
				}
			},
			componentInput: undefined,
			numberOfSubgrids: 1
		}
	},
	schemaformcomponent: {
		name: 'Form',
		icon: FileText,
		dims: '3:8-8:12' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			label: { class: '', style: '' },
			description: { class: '', style: '' }
		},
		initialData: {
			componentInput: {
				type: 'static',
				fieldType: 'schema',
				value: {
					properties: {
						first_name: {
							type: 'string',
							description: 'your name',
							default: 'default'
						}
					},
					required: []
				}
			},
			configuration: {
				displayType: {
					fieldType: 'boolean',
					type: 'static',
					value: false,
					onlyStatic: true,
					tooltip: 'This will diplay the type and/or the format on the field next to the label.'
				},
				largeGap: {
					fieldType: 'boolean',
					type: 'static',
					value: false,
					onlyStatic: true,
					tooltip: 'This will add a large gap between the form elements.'
				}
			}
		}
	},
	selecttabcomponent: {
		name: 'Select Tab',
		icon: List,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			tabRow: { class: '', style: '' },
			allTabs: { class: '', style: '' },
			selectedTab: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'labeledselect',
					value: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					]
				} as StaticAppInput,

				defaultValue: {
					type: 'static',
					value: undefined as { value: string; label: string } | undefined,
					fieldType: 'object'
				},
				tabSize: {
					type: 'static',
					value: 'sm',
					fieldType: 'select',
					selectOptions: selectOptions.buttonSizeOptions,
					tooltip:
						'Size of the tabs can be overwritten with custom styles using `font-size` in CSS or using tailwind classes.'
				}
			}
		}
	},
	selectstepcomponent: {
		name: 'Select Step',
		icon: List,
		dims: '2:1-3:1' as AppComponentDimensions,

		customCss: {
			tabRow: { class: '', style: '' },
			allTabs: { class: '', style: '' },
			selectedTab: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				items: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'labeledselect',
					value: [
						{ value: 'foo', label: 'Foo' },
						{ value: 'bar', label: 'Bar' }
					]
				} as StaticAppInput,
				defaultValue: {
					type: 'static',
					value: undefined as { value: string; label: string } | undefined,
					fieldType: 'object'
				}
			}
		}
	},
	conditionalwrapper: {
		name: 'Conditional tabs',
		icon: Split,
		dims: '2:8-6:8' as AppComponentDimensions,

		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: undefined,
			numberOfSubgrids: 2,
			conditions: [
				{
					type: 'eval',
					expr: 'false',
					fieldType: 'boolean'
				},
				{
					type: 'eval',
					expr: 'true',
					fieldType: 'boolean'
				}
			] as AppInputSpec<'boolean', boolean>[]
		}
	}
} as const

export const presetComponents = {
	sidebartabscomponent: {
		name: 'Sidebar Tabs',
		icon: PanelLeft,
		targetComponent: 'tabscomponent' as const,
		configuration: {
			tabsKind: {
				value: 'sidebar'
			}
		},
		type: 'sidebartabscomponent'
	},
	invisibletabscomponent: {
		name: 'Invisible Tabs',
		icon: PanelTopInactive,
		targetComponent: 'tabscomponent' as const,
		configuration: {
			tabsKind: {
				value: 'invisibleOnView'
			}
		},
		type: 'invisibletabscomponent'
	}
}

export const presets: {
	[Property in keyof typeof presetComponents]: PresetComponentConfig
} = presetComponents

export const ccomponents: {
	[Property in keyof typeof components]: AppComponentConfig<Property>
} = components

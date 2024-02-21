import type { IntRange } from '../../../../common'
import type { NARROW_GRID_COLUMNS, WIDE_GRID_COLUMNS } from '../../gridUtils'
import { BUTTON_COLORS } from '../../../common'
import type { ChartType } from 'chart.js'
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
	PanelTopInactive,
	ListIcon,
	Heading1,
	FileBarChart,
	Menu,
	Network,
	Database,
	UploadCloud,
	AlertTriangle,
	Clock,
	CalendarClock
} from 'lucide-svelte'
import type {
	Aligned,
	BaseAppComponent,
	ComponentCustomCSS,
	GridItem,
	RichConfiguration,
	RichConfigurations,
	StaticRichConfigurations
} from '../../types'
import type { Size } from '../../svelte-grid/types'

import type {
	AppInputSpec,
	EvalV2AppInput,
	InputConnectionEval,
	ResultAppInput,
	StaticAppInput,
	TemplateV2AppInput
} from '../../inputType'

export type BaseComponent<T extends string> = {
	type: T
}

export type RecomputeOthersSource = {
	recomputeIds: string[] | undefined
}

export type CustomComponentConfig = {
	name: string
	additionalLibs?: {
		reactVersion?: string
	}
}
export type TextComponent = BaseComponent<'textcomponent'>
export type TextInputComponent = BaseComponent<'textinputcomponent'>
export type QuillComponent = BaseComponent<'quillcomponent'>
export type TextareaInputComponent = BaseComponent<'textareainputcomponent'>
export type PasswordInputComponent = BaseComponent<'passwordinputcomponent'>
export type EmailInputComponent = BaseComponent<'emailinputcomponent'>
export type DateInputComponent = BaseComponent<'dateinputcomponent'>
export type TimeInputComponent = BaseComponent<'timeinputcomponent'>
export type DateTimeInputComponent = BaseComponent<'datetimeinputcomponent'>
export type NumberInputComponent = BaseComponent<'numberinputcomponent'>
export type CurrencyComponent = BaseComponent<'currencycomponent'>
export type SliderComponent = BaseComponent<'slidercomponent'>
export type DateSliderComponent = BaseComponent<'dateslidercomponent'>
export type RangeComponent = BaseComponent<'rangecomponent'>
export type HtmlComponent = BaseComponent<'htmlcomponent'>
export type CustomComponent = BaseComponent<'customcomponent'> & {
	customComponent: CustomComponentConfig
}
export type MarkdownComponent = BaseComponent<'mardowncomponent'>
export type VegaLiteComponent = BaseComponent<'vegalitecomponent'>
export type PlotlyComponent = BaseComponent<'plotlycomponent'>
export type PlotlyComponentV2 = BaseComponent<'plotlycomponentv2'> & {
	xData: RichConfiguration | undefined
	datasets: RichConfiguration | undefined
}
export type TimeseriesComponent = BaseComponent<'timeseriescomponent'>
export type ButtonComponent = BaseComponent<'buttoncomponent'> & RecomputeOthersSource
export type DownloadComponent = BaseComponent<'downloadcomponent'>
export type FormComponent = BaseComponent<'formcomponent'> & RecomputeOthersSource
export type FormButtonComponent = BaseComponent<'formbuttoncomponent'> & RecomputeOthersSource

export type RunFormComponent = BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = BaseComponent<'piechartcomponent'>
export type ChartJsComponent = BaseComponent<'chartjscomponent'>
export type ChartJsComponentV2 = BaseComponent<'chartjscomponentv2'> & {
	xData: RichConfiguration | undefined
	datasets: RichConfiguration | undefined
}

export type AgChartsComponent = BaseComponent<'agchartscomponent'> & {
	xData: RichConfiguration | undefined
	datasets: RichConfiguration | undefined
}

export type AgChartsComponentEe = BaseComponent<'agchartscomponentee'> & {
	license: string
	xData: RichConfiguration | undefined
	datasets: RichConfiguration | undefined
}

export type ScatterChartComponent = BaseComponent<'scatterchartcomponent'>

export type TableComponent = BaseComponent<'tablecomponent'> & {
	actionButtons: (BaseAppComponent & ButtonComponent & GridItem)[]
}
export type AggridComponent = BaseComponent<'aggridcomponent'>
export type AggridComponentEe = BaseComponent<'aggridcomponentee'> & {
	license: string
}
export type DisplayComponent = BaseComponent<'displaycomponent'>
export type LogComponent = BaseComponent<'logcomponent'>
export type JobIdLogComponent = BaseComponent<'jobidlogcomponent'>
export type FlowStatusComponent = BaseComponent<'flowstatuscomponent'>
export type JobIdFlowStatusComponent = BaseComponent<'jobidflowstatuscomponent'>
export type ImageComponent = BaseComponent<'imagecomponent'>
export type InputComponent = BaseComponent<'inputcomponent'>
export type SelectComponent = BaseComponent<'selectcomponent'> &
	RecomputeOthersSource & {
		onSelect?: string[]
	}
export type ResourceSelectComponent = BaseComponent<'resourceselectcomponent'> &
	RecomputeOthersSource & {
		onSelect?: string[]
	}
export type MultiSelectComponent = BaseComponent<'multiselectcomponent'>
export type CheckboxComponent = BaseComponent<'checkboxcomponent'> &
	RecomputeOthersSource & {
		onToggle?: string[]
	}
export type RadioComponent = BaseComponent<'radiocomponent'>
export type IconComponent = BaseComponent<'iconcomponent'>
export type HorizontalDividerComponent = BaseComponent<'horizontaldividercomponent'>
export type VerticalDividerComponent = BaseComponent<'verticaldividercomponent'>
export type FileInputComponent = BaseComponent<'fileinputcomponent'>
export type TabsComponent = BaseComponent<'tabscomponent'> & {
	tabs: string[]
	disabledTabs: RichConfiguration[]
}
export type ListComponent = BaseComponent<'listcomponent'>
export type ContainerComponent = BaseComponent<'containercomponent'> & {
	groupFields: RichConfigurations
}
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

export type CarouselListComponent = BaseComponent<'carousellistcomponent'>
export type StatisticCardComponent = BaseComponent<'statcomponent'>
export type MenuComponent = BaseComponent<'menucomponent'> & {
	menuItems: (BaseAppComponent & ButtonComponent & GridItem)[]
}

export type DBExplorerComponent = BaseComponent<'dbexplorercomponent'> & {
	columns: RichConfiguration
}

export type S3FileInputComponent = BaseComponent<'s3fileinputcomponent'>

export type DecisionTreeNode = {
	id: string
	label: string
	allowed: RichConfiguration | undefined
	next: Array<{
		id: string
		condition?: RichConfiguration | undefined
	}>
}

export type DecisionTreeComponent = BaseComponent<'decisiontreecomponent'> & {
	nodes: DecisionTreeNode[]
}

export type AlertComponent = BaseComponent<'alertcomponent'>

export type TypedComponent =
	| DBExplorerComponent
	| DisplayComponent
	| LogComponent
	| JobIdLogComponent
	| FlowStatusComponent
	| JobIdFlowStatusComponent
	| TextInputComponent
	| QuillComponent
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
	| CustomComponent
	| MarkdownComponent
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
	| ListComponent
	| IconComponent
	| HorizontalDividerComponent
	| VerticalDividerComponent
	| FileInputComponent
	| ImageComponent
	| AggridComponent
	| AggridComponentEe
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
	| ChartJsComponent
	| CarouselListComponent
	| PlotlyComponentV2
	| ChartJsComponentV2
	| StatisticCardComponent
	| MenuComponent
	| DecisionTreeComponent
	| S3FileInputComponent
	| AgChartsComponent
	| AgChartsComponentEe
	| AlertComponent
	| DateSliderComponent
	| TimeInputComponent
	| DateTimeInputComponent

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

export type Quickstyle = { quickCss?: string[]; quickTailwindClasses?: string[] }
export type AppComponentConfig<T extends TypedComponent['type']> = {
	name: string
	icon: any
	documentationLink: string
	quickstyle?: Record<string, Quickstyle>
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
	componentInput?: StaticAppInput | ResultAppInput | EvalV2AppInput | TemplateV2AppInput | undefined
	configuration: StaticRichConfigurations
	// Number of subgrids
	numberOfSubgrids?: number
	recomputeIds?: boolean
	actionButtons?: boolean
	menuItems?: boolean
	tabs?: string[]
	panes?: number[]
	conditions?: AppInputSpec<'boolean', boolean>[]
	nodes?: DecisionTreeNode[]
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
	formorientationOptions: ['Horizontal', 'Vertical'],
	chartTypeOptions: [
		'bar',
		'bubble',
		'doughnut',
		'line',
		'pie',
		'polarArea',
		'radar',
		'scatter'
	] as ChartType[],
	animationTimingFunctionOptions: ['linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out'],
	prose: ['sm', 'Default', 'lg', 'xl', '2xl'],
	imageSourceKind: [
		'url',
		'png encoded as base64',
		'jpeg encoded as base64',
		'svg encoded as base64'
	]
}
const labels = {
	none: 'Do nothing',
	errorOverlay: 'Show error overlay',
	gotoUrl: 'Go to an url',
	setTab: 'Set the tab of a tabs component',
	sendToast: 'Display a toast notification',
	sendErrorToast: 'Display an error toast notification',
	open: 'Open a modal or a drawer',
	close: 'Close a modal or a drawer',
	openModal: 'Open a modal (deprecated)',
	closeModal: 'Close a modal (deprecated)',
	clearFiles: 'Clear files from a S3 file input'
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
				placeholder: '/apps/get/foo',
				onDemandOnly: true
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
				tooltip: 'Set the tabs id and index to go to on success',
				onDemandOnly: true
			}
		},
		sendToast: {
			message: {
				tooltip: 'The message of the toast to diplay',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: 'Hello there',
				onDemandOnly: true
			}
		},
		openModal: {
			modalId: {
				tooltip: 'The id of the modal to open',
				fieldType: 'text',
				type: 'static',
				value: '',
				deprecated: true
			}
		},
		closeModal: {
			modalId: {
				tooltip: 'The id of the modal to close',
				fieldType: 'text',
				type: 'static',
				value: '',
				deprecated: true
			}
		},
		open: {
			id: {
				tooltip: 'The id of the modal or the drawer to open',
				fieldType: 'text',
				type: 'static',
				value: ''
			}
		},
		close: {
			id: {
				tooltip: 'The id of the modal or the drawer to close',
				fieldType: 'text',
				type: 'static',
				value: ''
			}
		},
		clearFiles: {
			id: {
				tooltip: 'The id of s3 file input to clear',
				fieldType: 'text',
				type: 'static',
				value: ''
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
				placeholder: '/apps/get/foo',
				onDemandOnly: true
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
				tooltip: 'Set the tabs id and index to go to on error',
				onDemandOnly: true
			}
		},
		sendErrorToast: {
			message: {
				tooltip: 'The message of the toast to diplay',
				fieldType: 'text',
				type: 'static',
				value: '',
				placeholder: 'Hello there',
				onDemandOnly: true
			},
			appendError: {
				tooltip: 'Append the error message to the toast',
				fieldType: 'boolean',
				type: 'static',
				value: true
			}
		},
		open: {
			id: {
				tooltip: 'The id of the modal or the drawer to open',
				fieldType: 'text',
				type: 'static',
				value: '',
				noVariablePicker: true
			}
		},
		close: {
			id: {
				tooltip: 'The id of the modal or the drawer to close',
				fieldType: 'text',
				type: 'static',
				value: '',
				noVariablePicker: true
			}
		}
	}
} as const

const paginationOneOf = {
	type: 'oneOf',
	selected: 'auto',
	labels: {
		auto: 'Managed by component',
		manual: 'Managed by runnable'
	},
	tooltip:
		'Pagination can be managed using two methods: By the component: Based on a specified page size, the component divides the array into several pages. By the runnable: The component shows all items, leaving the task of pagination to the runnable. The current page number is available in the component outputs.',
	configuration: {
		auto: {
			pageSize: {
				type: 'static',
				fieldType: 'number',
				value: 20,

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

const documentationBaseUrl = 'https://www.windmill.dev/docs/apps/app_configuration_settings'

const aggridcomponentconst = {
	name: 'AgGrid Table',
	icon: Table2,
	documentationLink: `${documentationBaseUrl}/aggrid_table`,
	dims: '3:10-6:10' as AppComponentDimensions,
	customCss: {
		container: { class: '', style: '' }
	},
	initialData: {
		configuration: {
			columnDefs: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'ag-grid',
				value: [
					{ field: 'id', flex: 1 },
					{ field: 'name', editable: true, flex: 1 },
					{ field: 'age', flex: 1 }
				]
			} as StaticAppInput,
			flex: {
				type: 'static',
				fieldType: 'boolean',
				value: true,

				tooltip: 'default col flex is 1 (see ag-grid docs)'
			},
			allEditable: {
				type: 'static',
				fieldType: 'boolean',
				value: false,
				hide: true,
				tooltip: 'Configure all columns as Editable by users'
			},
			multipleSelectable: {
				type: 'static',
				fieldType: 'boolean',
				value: false,

				tooltip: 'Make multiple rows selectable at once'
			},
			rowMultiselectWithClick: {
				type: 'static',
				fieldType: 'boolean',
				value: true,

				tooltip: 'If multiple selectable, allow multiselect with click'
			},
			pagination: {
				type: 'static',
				fieldType: 'boolean',
				value: false as boolean | undefined
			},
			selectFirstRowByDefault: {
				type: 'static',
				fieldType: 'boolean',
				value: true as boolean,
				tooltip: 'Select the first row by default on start'
			},
			extraConfig: {
				type: 'static',
				fieldType: 'object',
				value: {},
				tooltip: 'any configuration that can be passed to ag-grid top level'
			},
			compactness: {
				type: 'static',
				fieldType: 'select',
				value: 'normal',
				selectOptions: ['normal', 'compact', 'comfortable'],
				tooltip: 'Change the row height'
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
} as const

const agchartscomponentconst = {
	name: 'AgCharts',
	icon: BarChart4,
	documentationLink: `${documentationBaseUrl}/agcharts`,
	dims: '2:8-6:8' as AppComponentDimensions,
	customCss: {
		container: { class: '', style: '' }
	},
	initialData: {
		configuration: {},
		componentInput: undefined
	}
} as const

export const components = {
	displaycomponent: {
		name: 'Rich Result',
		icon: Monitor,
		documentationLink: `${documentationBaseUrl}/rich_result`,
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
			configuration: {
				title: {
					type: 'static',
					fieldType: 'text',
					value: 'Result'
				},
				hideDetails: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					tooltip:
						'Hide the details section: the object keys, the clipboard button and the maximise button'
				}
			}
		}
	},
	jobidlogcomponent: {
		name: 'Log by Job Id',
		icon: Monitor,
		documentationLink: `${documentationBaseUrl}/log_display`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			header: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				jobId: {
					type: 'static',
					fieldType: 'text',
					value: '',
					tooltip: 'Job id to display logs from'
				}
			}
		}
	},
	logcomponent: {
		name: 'Log',
		icon: Monitor,
		documentationLink: `${documentationBaseUrl}/log_display`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			header: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			}
		}
	},
	flowstatuscomponent: {
		name: 'Flow Status',
		icon: Monitor,
		documentationLink: `${documentationBaseUrl}/flow_status`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			header: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined
			}
		}
	},
	jobidflowstatuscomponent: {
		name: 'Flow Status by Job Id',
		icon: Monitor,
		documentationLink: `${documentationBaseUrl}/flow_status`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			header: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				jobId: {
					type: 'static',
					fieldType: 'text',
					value: '',
					tooltip: 'Job id to display logs from'
				}
			}
		}
	},
	containercomponent: {
		name: 'Container',
		icon: BoxSelect,
		documentationLink: `${documentationBaseUrl}/container`,
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
	listcomponent: {
		name: 'List',
		icon: ListIcon,
		documentationLink: `${documentationBaseUrl}/list`,
		dims: '3:8-12:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				width: {
					type: 'oneOf',
					selected: 'card',
					labels: {
						card: 'Card',
						row: 'Full Row'
					},
					configuration: {
						card: {
							minWidthPx: {
								type: 'static',
								fieldType: 'number',
								value: 300,
								tooltip: 'Min Width in pixels'
							}
						},
						row: {}
					}
				} as const,
				heightPx: {
					type: 'static',
					fieldType: 'number',
					value: undefined,
					tooltip: 'Height in pixels'
				},

				pagination: {
					type: 'oneOf',
					selected: 'auto',
					labels: {
						auto: 'Managed by component',
						manual: 'Managed by runnable'
					},
					tooltip:
						'Pagination can be managed using two methods: By the component: Based on a specified page size, the component divides the array into several pages. By the runnable: The component shows all items, leaving the task of pagination to the runnable. The current page number is available in the component outputs.',
					configuration: {
						manual: {
							pageCount: {
								type: 'static',
								fieldType: 'number',
								value: -1,
								tooltip: 'Number of pages (-1 if you do not know)'
							}
						},
						auto: {
							pageSize: {
								type: 'static',
								fieldType: 'number',
								value: 20,
								tooltip: 'Number of items per page'
							}
						}
					}
				} as const,
				displayBorders: {
					type: 'static',
					fieldType: 'boolean',
					value: true,
					tooltip: 'Display borders between items'
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
				value: [{ foo: 1 }, { foo: 2 }, { foo: 3 }] as object[]
			},
			numberOfSubgrids: 1
		}
	},
	textcomponent: {
		name: 'Text',
		icon: Type,
		dims: '1:1-3:1' as AppComponentDimensions,
		documentationLink: `${documentationBaseUrl}/text`,
		customCss: {
			text: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'left',
			verticalAlignment: 'top',

			componentInput: {
				type: 'templatev2',
				fieldType: 'template',
				eval: 'Hello ${ctx.username}',
				connections: [
					{
						id: 'username',
						componentId: 'ctx'
					}
				] as InputConnectionEval[]
			},
			configuration: {
				style: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.textStyleOptions,
					value: 'Body' as 'Title' | 'Subtitle' | 'Body' | 'Label' | 'Caption'
				},
				copyButton: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				tooltip: {
					type: 'static',
					value: '',
					fieldType: 'text',

					tooltip: 'Tooltip text if not empty'
				}
			}
		}
	},
	buttoncomponent: {
		name: 'Button',
		icon: Inspect,
		dims: '1:1-2:1' as AppComponentDimensions,
		documentationLink: `${documentationBaseUrl}/button`,
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

					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',

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
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				},
				triggerOnAppLoad: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},

				onSuccess: onSuccessClick,
				onError: onErrorClick,
				confirmationModal: {
					type: 'oneOf',
					selected: 'none',
					tooltip: 'If defined, the user will be asked to confirm the action in a modal.',
					labels: {
						none: 'Do nothing',
						confirmationModal: 'Show confirmation modal'
					},
					configuration: {
						none: {},
						confirmationModal: {
							title: {
								fieldType: 'text',
								type: 'static',
								value: 'Title',
								placeholder: 'Confirmation modal title'
							},
							description: {
								fieldType: 'text',
								type: 'static',
								value: 'Are you sure?',
								placeholder: 'Are you sure?'
							},
							confirmationText: {
								fieldType: 'text',
								type: 'static',
								value: 'Confirm',
								placeholder: 'Confirm',
								tooltip: 'The text of the button that confirms the action.'
							}
						}
					}
				}
			}
		}
	},
	downloadcomponent: {
		name: 'Download Button',
		icon: Download,
		documentationLink: `${documentationBaseUrl}/download_button`,
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
					placeholder: 'Enter URL or upload file (base64)'
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

					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',

					value: false
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				}
			}
		}
	},
	formcomponent: {
		name: 'Submit form',
		icon: FormInput,
		documentationLink: `${documentationBaseUrl}/submit_form`,
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
		documentationLink: `${documentationBaseUrl}/modal_form`,
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
				modalTitle: {
					type: 'static',
					fieldType: 'text',
					value: 'Modal title'
				},
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
		documentationLink: `${documentationBaseUrl}/pie_chart`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				theme: {
					type: 'static',

					fieldType: 'select',
					selectOptions: selectOptions.chartThemeOptions,
					value: 'theme1'
				},
				doughnutStyle: {
					type: 'static',

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
	chartjscomponent: {
		name: 'ChartJs',
		icon: PieChart,
		documentationLink: `${documentationBaseUrl}/chartjs`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				type: {
					type: 'static',

					fieldType: 'select',
					selectOptions: selectOptions.chartTypeOptions,
					value: 'pie'
				},
				options: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip: 'ChartJs options object https://www.chartjs.org/docs/latest/general/options.html'
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: {
					labels: ['Pie', 'Charts', '<3'],
					datasets: [
						{
							data: [25, 50, 25],
							backgroundColor: ['#FF6384', '#4BC0C0', '#FFCE56']
						}
					]
				}
			}
		}
	},
	chartjscomponentv2: {
		name: 'ChartJs',
		icon: PieChart,
		documentationLink: `${documentationBaseUrl}/chartjs`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				type: {
					type: 'static',

					fieldType: 'select',
					selectOptions: selectOptions.chartTypeOptions,
					value: 'pie'
				},
				options: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip: 'ChartJs options object https://www.chartjs.org/docs/latest/general/options.html'
				}
			},
			componentInput: undefined
		}
	},
	barchartcomponent: {
		name: 'Bar/Line Chart',
		icon: BarChart4,
		documentationLink: `${documentationBaseUrl}/bar_line_chart`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				theme: {
					type: 'static',
					fieldType: 'select',
					selectOptions: selectOptions.chartThemeOptions,
					value: 'theme1'
				},
				line: {
					type: 'static',

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

	agchartscomponent: agchartscomponentconst,
	agchartscomponentee: { ...agchartscomponentconst, name: 'AgCharts EE' },
	htmlcomponent: {
		name: 'HTML',
		icon: Code2,
		documentationLink: `${documentationBaseUrl}/html`,
		dims: '1:2-1:2' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: {
				type: 'templatev2',
				fieldType: 'template',
				eval: `<img
src="https://images.unsplash.com/photo-1554629947-334ff61d85dc?ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&amp;ixlib=rb-1.2.1&amp;auto=format&amp;fit=crop&amp;w=1024&amp;h=1280&amp;q=80"
>
<h1 class="absolute top-4 left-2 text-white">
Hello \${ctx.username}
</h1>`,
				connections: [
					{
						id: 'username',
						componentId: 'ctx'
					}
				] as InputConnectionEval[]
			},
			configuration: {}
		}
	},
	customcomponent: {
		name: 'Custom',
		icon: Code2,
		documentationLink: `https://www.windmill.dev/docs/apps/react_components`,
		dims: '1:2-1:2' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: {
				type: 'static',
				fieldType: 'object',
				value: {}
			},
			configuration: {}
		}
	},
	mardowncomponent: {
		name: 'Markdown',
		icon: Heading1,
		documentationLink: `${documentationBaseUrl}/html`,
		dims: '1:2-4-4' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			componentInput: {
				type: 'templatev2',
				fieldType: 'template',
				eval: `# This is a header
## This is a subheader				
This is a paragraph.
				
* This is a list
* With two items`,
				connections: [] as InputConnectionEval[]
			},
			configuration: {
				size: {
					fieldType: 'select',
					type: 'static',
					selectOptions: selectOptions.prose,
					value: 'Default',

					tooltip: 'See Tailwind documentation: https://tailwindcss.com/docs/typography-plugin'
				}
			}
		}
	},
	vegalitecomponent: {
		name: 'Vega Lite',
		icon: PieChart,
		documentationLink: `${documentationBaseUrl}/vega_lite`,
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
		documentationLink: `${documentationBaseUrl}/plotly`,
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
						'Layout options for the plot. See https://plotly.com/javascript/reference/layout/ for more information'
				}
			}
		}
	},
	plotlycomponentv2: {
		name: 'Plotly',
		icon: PieChart,
		documentationLink: `${documentationBaseUrl}/plotly`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {},
		initialData: {
			componentInput: undefined,
			configuration: {
				layout: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip:
						'Layout options for the plot. See https://plotly.com/javascript/reference/layout/ for more information'
				}
			}
		}
	},
	timeseriescomponent: {
		name: 'Timeseries',
		icon: GripHorizontal,
		documentationLink: `${documentationBaseUrl}/timeseries`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				logarithmicScale: {
					type: 'static',

					fieldType: 'boolean',
					value: false
				},
				zoomable: {
					type: 'static',

					fieldType: 'boolean',
					value: false
				},
				pannable: {
					type: 'static',

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
		documentationLink: `${documentationBaseUrl}/scatter_chart`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				zoomable: {
					type: 'static',

					fieldType: 'boolean',
					value: false
				},
				pannable: {
					type: 'static',

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
		documentationLink: `${documentationBaseUrl}/table`,
		dims: '3:10-6:10' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			tableHeader: { class: '', style: '' },
			tableBody: { class: '', style: '' },
			tableFooter: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				columnDefs: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'table-column',
					value: [{ field: 'id' }, { field: 'name' }, { field: 'age' }]
				} as StaticAppInput,
				search: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.tableSearchOptions,
					value: 'Disabled' as string,
					tooltip:
						'Search can be configured in the following ways: Disabled: The search is disabled,By Runnable: The search is done in the backend, or by component: The search is done in the frontend.'
				},
				pagination: paginationOneOf,
				downloadButton: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip: 'display a button to download the table as a csv file'
				},
				initialState: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip:
						'any configuration that can be passed to the tanstack table component as initial state (https://tanstack.com/table/v8/docs/api/core/table#initialstate)'
				},
				selectFirstRowByDefault: {
					type: 'static',
					fieldType: 'boolean',
					value: true as boolean,
					tooltip: 'Select the first row by default on start'
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
			} as StaticAppInput,
			actionButtons: true
		}
	},
	aggridcomponent: aggridcomponentconst,
	aggridcomponentee: { ...aggridcomponentconst, name: 'AgGrid Table EE' },
	checkboxcomponent: {
		name: 'Toggle',
		icon: ToggleLeft,
		documentationLink: `${documentationBaseUrl}/toggle`,
		dims: '1:1-2:1' as AppComponentDimensions,
		customCss: {
			text: { class: '', style: '' },
			container: { class: '', style: '' }
		},
		initialData: {
			...defaultAlignement,
			componentInput: undefined,
			onToggle: [],
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
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			}
		}
	},
	textinputcomponent: {
		name: 'Text Input',
		icon: TextCursorInput,
		documentationLink: `${documentationBaseUrl}/text_input`,
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
					fieldType: 'text'
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'text'
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				}
			}
		}
	},
	quillcomponent: {
		name: 'Rich Text Editor',
		icon: TextCursorInput,
		documentationLink: `${documentationBaseUrl}/rich_text_editor`,
		dims: '2:1-4:4' as AppComponentDimensions,
		customCss: {},
		initialData: {
			componentInput: undefined,
			configuration: {
				placeholder: {
					type: 'static',
					value: 'Type...',
					fieldType: 'text'
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
		documentationLink: `${documentationBaseUrl}/textarea`,
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
					fieldType: 'text'
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'text'
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			}
		}
	},
	selectcomponent: {
		name: 'Select',
		icon: List,
		documentationLink: `${documentationBaseUrl}/select`,
		dims: '2:1-3:1' as AppComponentDimensions,
		customCss: {
			input: {
				style: '',
				tooltip:
					'https://github.com/rob-balfre/svelte-select/blob/master/docs/theming_variables.md',
				class: ''
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

					tooltip: 'Allows user to manually add new value',
					customTitle: 'User creatable'
				},
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select an item'
				},
				disabled: {
					fieldType: 'boolean',
					type: 'static',
					value: false
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'object'
				},
				preselectFirst: {
					type: 'static',
					value: true,
					fieldType: 'boolean',

					tooltip: 'Preselect first item in the options if no default value is set'
				},
				fullWidth: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip: 'Set the width of the options popup to 100% of the select width'
				}
			}
		}
	},
	multiselectcomponent: {
		name: 'Multi Select',
		icon: List,
		documentationLink: `${documentationBaseUrl}/multiselect`,
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
				defaultItems: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'text',
					value: []
				} as StaticAppInput,
				placeholder: {
					type: 'static',
					fieldType: 'text',
					value: 'Select items'
				},
				create: {
					type: 'static',
					fieldType: 'boolean',
					value: false,

					tooltip: 'Allows user to manually add new value',
					customTitle: 'User creatable'
				},
				allowOverflow: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip:
						'If too many items, the box overflow its container instead of having an internal scroll'
				}
			}
		}
	},
	resourceselectcomponent: {
		name: 'Resource Select',
		icon: List,
		documentationLink: `${documentationBaseUrl}/resource_select`,
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
					value: 'Select an item'
				},
				fullWidth: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip: 'Set the width of the options popup to 100% of the select width'
				}
			}
		}
	},
	numberinputcomponent: {
		name: 'Number',
		icon: Binary,
		documentationLink: `${documentationBaseUrl}/number_input`,
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
					fieldType: 'text'
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
		documentationLink: `${documentationBaseUrl}/currency_input`,
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
					fieldType: 'boolean'
				},
				currency: {
					type: 'static',
					value: 'USD',
					fieldType: 'select',

					selectOptions: selectOptions.currencyOptions
				},
				locale: {
					type: 'static',
					value: 'en-US',
					fieldType: 'select',

					selectOptions: selectOptions.localeOptions,
					tooltip: 'Currency format'
				}
			}
		}
	},
	slidercomponent: {
		name: 'Slider',
		icon: SlidersHorizontal,
		documentationLink: `${documentationBaseUrl}/slider`,
		dims: '3:1-4:1' as AppComponentDimensions,
		customCss: {
			bar: { style: '', class: '' },
			handle: { style: '', class: '' },
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
					fieldType: 'number'
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
				},
				vertical: {
					type: 'static',
					fieldType: 'boolean',
					value: false
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			}
		}
	},
	dateslidercomponent: {
		name: 'Date Slider',
		icon: SlidersHorizontal,
		documentationLink: `${documentationBaseUrl}/date_slider`,
		dims: '3:1-4:1' as AppComponentDimensions,
		customCss: {
			bar: { style: '', class: '' },
			handle: { style: '', class: '' },
			limits: { class: '', style: '' },
			value: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				min: {
					type: 'static',
					value: '',
					fieldType: 'date'
				},
				max: {
					type: 'static',
					value: '',
					fieldType: 'date'
				},
				defaultValue: {
					type: 'static',
					value: '',
					fieldType: 'date'
				},
				step: {
					type: 'static',
					value: 1,
					fieldType: 'number',
					tooltip: 'Number of days between each date suggestion'
				},
				vertical: {
					type: 'static',
					fieldType: 'boolean',
					value: false
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				outputFormat: {
					type: 'static',
					value: undefined,
					fieldType: 'text',
					tooltip: 'See date-fns format for more information',
					documentationLink: 'https://date-fns.org/v1.29.0/docs/format'
				}
			}
		}
	},
	rangecomponent: {
		name: 'Range',
		icon: SlidersHorizontal,
		documentationLink: `${documentationBaseUrl}/range`,
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
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean',

					tooltip:
						'Determine if the slider is disabled, or enabled (only disables interactions, and events)'
				}
			}
		}
	},
	passwordinputcomponent: {
		name: 'Password',
		icon: Lock,
		documentationLink: `${documentationBaseUrl}/password_input`,
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
					fieldType: 'text'
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				}
			}
		}
	},
	emailinputcomponent: {
		name: 'Email Input',
		icon: AtSignIcon,
		documentationLink: `${documentationBaseUrl}/email_input`,
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
					fieldType: 'text'
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'text'
				},
				disabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				}
			}
		}
	},
	dateinputcomponent: {
		name: 'Date',
		icon: Calendar,
		documentationLink: `${documentationBaseUrl}/date_input`,
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
					fieldType: 'date',
					tooltip: 'The minimum date that can be selected. The format is: "yyyy-MM-dd"'
				},
				maxDate: {
					type: 'static',
					value: '',
					fieldType: 'date',
					tooltip: 'The maximum date that can be selected. The format is: "yyyy-MM-dd"'
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'date'
				},
				outputFormat: {
					type: 'static',
					value: undefined,
					fieldType: 'text',
					markdownTooltip: `### Output format				
See date-fns format for more information. By default, it is 'dd.MM.yyyy'

| Format      | Result | Description |
| ----------- | ----------- | ----------- |
| DD 				| 01, 02, ..., 31 | Day of the month |
| D 				| 1, 2, ..., 31 | Day of the month |
| MM 				| 01, 02, ..., 12 | Month |
| MMM 				| Jan, Feb, ..., Dec | Month |
| MMMM 				| January, February, ..., December | Month |
| YYYY 				| 2021, 2022, ... | Year |
`,

					documentationLink: 'https://date-fns.org/v1.29.0/docs/format',
					placeholder: 'dd.MM.yyyy'
				}
			}
		}
	},
	datetimeinputcomponent: {
		name: 'Date & Time',
		icon: CalendarClock,
		documentationLink: `${documentationBaseUrl}/datetime_input`,
		dims: '2:1-6:2' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				displayPresets: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					tooltip: 'Display presets to select the date for example, in 1 week, in 1 month, etc.'
				},
				minDateTime: {
					type: 'static',
					value: '',
					fieldType: 'datetime',
					tooltip:
						'The minimum date that can be selected. The format is the ISO 8601 format: "yyyy-MM-ddTHH:mm:ss:SSSZ", for example "2021-11-06T23:39:30.000Z", or toISOString() from a Date'
				},
				maxDateTime: {
					type: 'static',
					value: '',
					fieldType: 'datetime',
					tooltip:
						'The maximum date that can be selected. The format is the ISO 8601 format: "yyyy-MM-ddTHH:mm:ss:SSSZ", for example "2021-11-06T23:39:30.000Z", or toISOString() from a Date'
				},
				outputFormat: {
					type: 'static',
					value: undefined,
					fieldType: 'text',
					documentationLink: 'https://date-fns.org/v1.29.0/docs/format',
					placeholder: 'dd.MM.yyyy HH:mm',
					markdownTooltip: `### Output format				
See date-fns format for more information. By default, it is 'dd.MM.yyyy HH:mm'

| Format      | Result | Description |
| ----------- | ----------- | ----------- |
| DD 				| 01, 02, ..., 31 | Day of the month |
| D 				| 1, 2, ..., 31 | Day of the month |
| MM 				| 01, 02, ..., 12 | Month |
| MMM 				| Jan, Feb, ..., Dec | Month |
| MMMM 				| January, February, ..., December | Month |
| YYYY 				| 2021, 2022, ... | Year |
| HH 				| 00, 01, ..., 23 | Hours |
| mm 				| 00, 01, ..., 59 | Minutes |
| ss 				| 00, 01, ..., 59 | Seconds |
					`
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'datetime'
				}
			}
		}
	},
	timeinputcomponent: {
		name: 'Time',
		icon: Clock,
		documentationLink: `${documentationBaseUrl}/time_input`,
		dims: '2:1-3:1' as AppComponentDimensions,
		customCss: {
			input: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				minTime: {
					type: 'static',
					value: '',
					fieldType: 'time',
					tooltip:
						'The minimum date that can be selected. If the time provided is not valid, it will set the output "validity" to false. The format is: "HH:mm"'
				},
				maxTime: {
					type: 'static',
					value: '',
					fieldType: 'time',
					tooltip:
						'The maximum date that can be selected. If the time provided is not valid, it will set the output "validity" to false. The format is: "HH:mm"'
				},
				defaultValue: {
					type: 'static',
					value: undefined,
					fieldType: 'time'
				},

				['24hFormat']: {
					type: 'static',
					value: true,
					fieldType: 'boolean',
					tooltip:
						'Use 24h format. Will change the format of the output of the component: HH:mm to hh:mm am/pm'
				}
			}
		}
	},
	tabscomponent: {
		name: 'Tabs',
		icon: ListOrdered,
		documentationLink: `${documentationBaseUrl}/tabs`,
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
		documentationLink: `${documentationBaseUrl}/stepper`,
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
	carousellistcomponent: {
		name: 'Carousel List',
		icon: ListIcon,
		documentationLink: `${documentationBaseUrl}/carousel`,
		dims: '3:8-12:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				timingFunction: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.animationTimingFunctionOptions,
					value: 'linear',
					tooltip:
						'Sets how an animation progresses through the duration of each cycle, see https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function'
				}
			},
			componentInput: {
				type: 'static',
				fieldType: 'array',
				subFieldType: 'object',
				value: [{ foo: 1 }, { foo: 2 }, { foo: 3 }] as object[]
			},
			numberOfSubgrids: 1
		}
	},
	iconcomponent: {
		name: 'Icon',
		icon: Smile,
		documentationLink: `${documentationBaseUrl}/icon`,
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
					fieldType: 'number'
				},
				strokeWidth: {
					type: 'static',
					value: 2,
					fieldType: 'number'
				}
			}
		}
	},
	horizontaldividercomponent: {
		name: 'Divider X',
		icon: SeparatorHorizontal,
		documentationLink: `${documentationBaseUrl}/divider_x`,
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

					tooltip:
						'The height of the divider in pixels can be overridden by the `height` property in the styling menu'
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'color',

					tooltip:
						'The color of the divider can be overridden by the `background-color` property in the styling menu'
				}
			}
		}
	},
	verticaldividercomponent: {
		name: 'Divider Y',
		icon: SeparatorVertical,
		documentationLink: `${documentationBaseUrl}/divider_y`,
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

					tooltip:
						'The width of the divider in pixels can be overridden by the `width` property in the styling menu'
				},
				color: {
					type: 'static',
					value: '#00000060',
					fieldType: 'color',

					tooltip:
						'The color of the divider can be overridden by the `background-color` property in the styling menu'
				}
			}
		}
	},
	fileinputcomponent: {
		name: 'File Input',
		icon: Paperclip,
		documentationLink: `${documentationBaseUrl}/file_input`,
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
					fieldType: 'array'
				},
				allowMultiple: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					tooltip: 'If allowed, the user will be able to select more than one file'
				},
				text: {
					type: 'static',
					value: 'Drag and drop files or click to select them',
					fieldType: 'text'
				},
				includeMimeType: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					tooltip: 'If enabled, the mime type of the file will be included.'
				},
				submittedFileText: {
					type: 'static',
					value: 'Selected file',
					fieldType: 'text'
				}
			}
		}
	},
	imagecomponent: {
		name: 'Image',
		icon: Image,
		documentationLink: `${documentationBaseUrl}/image`,
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
				sourceKind: {
					fieldType: 'select',
					type: 'static',
					selectOptions: selectOptions.imageSourceKind,
					value: 'url' as (typeof selectOptions.imageSourceKind)[number]
				},
				imageFit: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.objectFitOptions,
					value: 'contain',
					tooltip:
						'The image fit property can be overridden by the `object-fit` property in the styling menu'
				},
				altText: {
					type: 'static',
					value: '',
					fieldType: 'text',

					tooltip: "This text will appear if the image can't be loaded for any reason"
				}
			}
		}
	},
	drawercomponent: {
		name: 'Drawer',
		icon: SidebarClose,
		documentationLink: `${documentationBaseUrl}/drawer`,
		dims: '1:1-2:1' as AppComponentDimensions,
		customCss: {
			button: { style: '', class: '' },
			container: { class: '', style: '' },
			drawer: { class: '', style: '' }
		},
		initialData: {
			horizontalAlignment: 'center',
			verticalAlignment: 'center',
			configuration: {
				drawerTitle: {
					type: 'static',
					fieldType: 'text',
					value: 'Drawer title'
				},
				hideButtonOnView: {
					fieldType: 'boolean',
					type: 'static',
					value: false,
					tooltip: 'Make button invisible when app is used outside of the edit mode'
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

					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',

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
		documentationLink: `${documentationBaseUrl}/map`,
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
		documentationLink: `${documentationBaseUrl}/vertical_split_panes`,
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
		documentationLink: `${documentationBaseUrl}/horizontal_split_panes`,
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
		documentationLink: `${documentationBaseUrl}/pdf`,
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
					placeholder: 'Enter URL or upload file (base64)'
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
		documentationLink: `${documentationBaseUrl}/modal`,
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
					value: 'Modal title'
				},
				hideButtonOnView: {
					fieldType: 'boolean',
					type: 'static',
					value: false,
					tooltip: 'Make button invisible when app is used outside of the edit mode'
				},
				buttonLabel: {
					type: 'static',
					fieldType: 'text',
					value: 'Press me'
				},
				buttonColor: {
					fieldType: 'select',
					type: 'static',

					selectOptions: buttonColorOptions,
					value: 'blue',
					tooltip: 'Theses presets can be overwritten with custom styles.'
				},
				buttonSize: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				buttonFillContainer: {
					fieldType: 'boolean',
					type: 'static',

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
		documentationLink: `${documentationBaseUrl}/form_input`,
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
				defaultValues: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip:
						'This enables setting default form values dynamically using an object: keys are field names, and values are the defaults.'
				},
				dynamicEnums: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip:
						'This enables setting form enum values dynamically using an object: keys are field names, and values are arrays of strings.'
				},

				displayType: {
					fieldType: 'boolean',
					type: 'static',
					value: false,

					tooltip: 'This will diplay the type and/or the format on the field next to the label.'
				},
				largeGap: {
					fieldType: 'boolean',
					type: 'static',
					value: false,

					tooltip: 'This will add a large gap between the form elements.'
				}
			}
		}
	},
	selecttabcomponent: {
		name: 'Select Tab',
		icon: List,
		documentationLink: `${documentationBaseUrl}/select_tab`,
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
					value: undefined,
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
		documentationLink: `${documentationBaseUrl}/select_step`,
		dims: '2:1-3:1' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
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
		documentationLink: `${documentationBaseUrl}/conditional_tabs`,
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
					type: 'evalv2',
					expr: 'false',
					fieldType: 'boolean',
					connections: []
				},
				{
					type: 'evalv2',
					expr: 'true',
					fieldType: 'boolean',
					connections: []
				}
			] as AppInputSpec<'boolean', boolean>[]
		}
	},
	statcomponent: {
		name: 'Statistic card',
		icon: FileBarChart,
		documentationLink: `${documentationBaseUrl}/statistic_card`,
		dims: '2:4-3:4' as AppComponentDimensions,
		quickstyle: {
			title: {
				quickCss: ['font-size: 1rem', 'font-size: 1.5rem', 'font-size: 2rem'],
				quickTailwindClasses: ['text-xs', 'text-sm', 'text-base', 'text-lg', 'text-xl', 'text-2xl']
			},
			value: {
				quickCss: ['font-size: 1rem', 'font-size: 1.5rem', 'font-size: 2rem'],
				quickTailwindClasses: ['text-xs', 'text-sm', 'text-base', 'text-lg', 'text-xl', 'text-2xl']
			}
		} as Record<string, Quickstyle>,
		customCss: {
			title: {
				class: '',
				style: ''
			},
			container: { class: '', style: '' },
			value: {
				class: '',
				style: ''
			},
			media: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				title: {
					type: 'static',
					value: 'Title',
					fieldType: 'text'
				},
				value: {
					type: 'static',
					value: 'Value',
					fieldType: 'text'
				},
				progress: {
					type: 'static',
					value: 0,
					fieldType: 'number'
				},
				media: {
					type: 'oneOf',
					selected: 'image',
					labels: {
						icon: 'Icon',
						image: 'Image'
					},
					configuration: {
						icon: {
							icon: {
								type: 'static',
								value: undefined,
								fieldType: 'icon-select'
							}
						},
						image: {
							source: {
								type: 'static',
								value: '/logo.svg',
								fieldType: 'text',
								fileUpload: {
									accept: 'image/*',
									convertTo: 'base64'
								}
							},
							sourceKind: {
								fieldType: 'select',
								type: 'static',
								selectOptions: selectOptions.imageSourceKind,
								value: 'url' as (typeof selectOptions.imageSourceKind)[number]
							}
						}
					}
				} as const
			}
		}
	},
	menucomponent: {
		name: 'Dropdown Menu',
		icon: Menu,
		documentationLink: `${documentationBaseUrl}/dropdown_menu`,
		dims: '1:1-1:2' as AppComponentDimensions,
		customCss: {
			button: { style: '', class: '' }
		},
		initialData: {
			...defaultAlignement,
			componentInput: undefined,
			configuration: {
				label: {
					type: 'static',
					fieldType: 'text',
					value: '' as string
				},
				color: {
					fieldType: 'select',
					type: 'static',
					selectOptions: selectOptions.buttonColorOptions,
					value: 'light'
				},
				size: {
					fieldType: 'select',
					type: 'static',

					selectOptions: selectOptions.buttonSizeOptions,
					value: 'xs'
				},
				fillContainer: {
					fieldType: 'boolean',
					type: 'static',

					value: false
				},
				beforeIcon: {
					type: 'static',
					value: 'Menu',
					fieldType: 'icon-select'
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select'
				}
			},
			menuItems: true
		}
	},
	decisiontreecomponent: {
		name: 'Decision Tree',
		icon: Network,
		documentationLink: `${documentationBaseUrl}/decision_tree`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {},
			componentInput: undefined,
			numberOfSubgrids: 1,
			nodes: [
				{
					id: 'a',
					label: 'a',
					allowed: {
						type: 'evalv2',
						expr: 'true',
						fieldType: 'boolean',
						connections: []
					},
					next: []
				}
			] as DecisionTreeNode[]
		}
	},
	s3fileinputcomponent: {
		name: 'S3 File Uploader',
		icon: UploadCloud,
		documentationLink: `${documentationBaseUrl}/s3fileinput`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				type: {
					type: 'oneOf',
					selected: 's3',
					labels: {
						s3: 'S3'
					},
					configuration: {
						s3: {
							resource: {
								type: 'static',
								fieldType: 'resource',
								value: '',
								subFieldType: 's3'
							} as StaticAppInput,
							acceptedFileTypes: {
								type: 'static',
								value: ['image/*', 'application/pdf'] as string[],
								fieldType: 'array'
							},
							allowMultiple: {
								type: 'static',
								value: false,
								fieldType: 'boolean',
								tooltip: 'If allowed, the user will be able to select more than one file'
							},
							text: {
								type: 'static',
								value: 'Drag and drop files or click to select them',
								fieldType: 'text'
							},
							/*
							displayDirectLink: {
								type: 'static',
								value: false,
								fieldType: 'boolean'
							},
							*/
							pathTemplate: {
								type: 'evalv2',
								expr: `\`\${file.name}\``,
								fieldType: 'template',
								connections: [],
								onDemandOnly: true
							} as EvalV2AppInput
						}
					}
				} as const
			}
		}
	},
	dbexplorercomponent: {
		name: 'Database Studio Table',
		icon: Database,
		documentationLink: `${documentationBaseUrl}/database_studio`,
		dims: '2:8-6:8' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' }
		},
		initialData: {
			configuration: {
				type: {
					type: 'oneOf',
					selected: 'postgresql',
					labels: {
						postgresql: 'PostgreSQL',
						msql: 'MySQL'
					},
					configuration: {
						postgresql: {
							resource: {
								type: 'static',
								fieldType: 'resource',
								value: ''
							} as StaticAppInput,
							table: {
								fieldType: 'select',
								subfieldType: 'db-table',
								type: 'static',
								selectOptions: [],
								value: undefined
							}
						}
					}
				} as const,
				columnDefs: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'db-explorer',
					value: []
				} as StaticAppInput,
				whereClause: {
					type: 'static',
					fieldType: 'text',
					value: ''
				},
				flex: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip: 'default col flex is 1 (see ag-grid docs)'
				},
				allEditable: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					hide: true,
					tooltip: 'Configure all columns as Editable by users'
				},
				allowDelete: {
					type: 'static',
					fieldType: 'boolean',
					value: false,
					hide: true,
					tooltip: 'Allow deleting rows'
				},
				multipleSelectable: {
					type: 'static',
					fieldType: 'boolean',
					value: false,

					tooltip: 'Make multiple rows selectable at once'
				},
				rowMultiselectWithClick: {
					type: 'static',
					fieldType: 'boolean',
					value: true,

					tooltip: 'If multiple selectable, allow multiselect with click'
				},
				selectFirstRowByDefault: {
					type: 'static',
					fieldType: 'boolean',
					value: true as boolean,
					tooltip: 'Select the first row by default on start'
				},
				extraConfig: {
					type: 'static',
					fieldType: 'object',
					value: {},
					tooltip: 'any configuration that can be passed to ag-grid top level'
				}
			},
			componentInput: undefined
		}
	},
	alertcomponent: {
		name: 'Alert',
		icon: AlertTriangle,
		documentationLink: `${documentationBaseUrl}/alert`,
		dims: '2:1-4:5' as AppComponentDimensions,
		customCss: {
			container: { class: '', style: '' },
			background: { class: '', style: '' },
			icon: { class: '', style: '' },
			title: { class: '', style: '' },
			description: { class: '', style: '' }
		},
		initialData: {
			verticalAlignment: 'center',
			componentInput: undefined,
			configuration: {
				type: {
					fieldType: 'select',
					type: 'static',
					selectOptions: [
						{ value: 'info', label: 'Info' },
						{ value: 'success', label: 'Success' },
						{ value: 'warning', label: 'Warning' },
						{ value: 'error', label: 'Error' }
					],
					value: 'info'
				},
				title: {
					type: 'static',
					value: 'Title',
					fieldType: 'text'
				},
				description: {
					type: 'static',
					value: 'Description',
					fieldType: 'text'
				},
				notRounded: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				tooltip: {
					type: 'static',
					value: '',
					fieldType: 'text'
				},
				size: {
					type: 'static',
					value: 'sm',
					fieldType: 'select',
					selectOptions: [
						{ value: 'xs', label: 'Extra Small' },
						{ value: 'sm', label: 'Small' }
					]
				},
				collapsible: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				initiallyCollapsed: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			}
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

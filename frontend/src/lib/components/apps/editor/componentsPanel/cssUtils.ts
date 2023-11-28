import type { ComponentCssProperty } from '../../types'

export const authorizedClassnames = [
	'wm-container',
	'wm-list',
	'wm-list-pagination',
	'wm-list-pagination-buttons',
	'wm-drawer',
	'wm-drawer-button',
	'wm-drawer-button-container',
	'wm-button',
	'wm-button-wrapper',
	'wm-button-container',
	'wm-vertical-split-panes',
	'wm-horizontal-split-panes',
	'wm-modal',
	'wm-modal-button',
	'wm-modal-button-container',
	'wm-tabs-container',
	'wm-tabs-tabRow',
	'wm-tabs-alltabs',
	'wm-tabs-selectedTab',
	'wm-carousel',
	'wm-submit',
	'wm-submit-button',
	'wm-number-input',
	'wm-currency-input',
	'wm-date-input',
	'wm-text-input',
	'wm-html',
	'wm-table-container',
	'wm-table-header',
	'wm-table-body',
	'wm-table-footer',
	'wm-table-row-selected',
	'wm-table-row',
	'wm-stepper',
	'wm-file-input',
	'wm-toggle-text',
	'wm-toggle-container',
	'wm-image',
	'wm-pdf',
	'wm-horizontal-divider',
	'wm-vertical-divider',
	'wm-horizontal-divider-container',
	'wm-vertical-divider-container',
	'wm-log-header',
	'wm-log-container',
	'wm-map',
	'wm-icon',
	'wm-icon-container',
	'wm-flow-status-header',
	'wm-flow-status-container',
	'wm-select-tab-row',
	'wm-select-tab',
	'wm-select-tab-selected',

	'ql-toolbar',
	'ql-stroke',
	'ql-fill',
	'ql-container',

	'wm-pie-chart',

	'wm-modal-form-popup',
	'wm-modal-form-button',

	'wm-download-button',
	'wm-download-button-container',

	'wm-bar-chart',
	'wm-scatter-chart',
	'wm-chartjs',
	'wm-timeseries',
	'wm-conditional-tabs',

	'wm-rich-result-header',
	'wm-rich-result-container'
	// TODO: Select and mutltiselect
]

interface Selector {
	selector: string
	comment?: string | undefined
	customCssKey?: string | undefined
}

interface Variable {
	variable: string
	value: string
	comment?: string | undefined
}

interface Customisation {
	components: string[]
	selectors: Selector[]
	variables: Variable[]
	link?: string | undefined
	variablesTooltip?: string
	root?: string
}

export const customisationByComponent: Customisation[] = [
	{
		components: ['app'],
		selectors: [
			{ selector: '.wm-app-viewer', comment: 'Applied to the div under all components' },
			{
				selector: '.wm-app-grid',
				comment: 'Applied to the div that contains the grid of components'
			},
			{
				selector: '.wm-app-component',
				comment: 'Applied to the div that is around every components'
			}
		],
		variables: []
	},
	{
		components: ['buttoncomponent'],
		selectors: [
			{ selector: '.wm-button', comment: 'Applied to the button', customCssKey: 'button' },
			{ selector: '.wm-button-wrapper', comment: 'Applied to the div around the button' },
			{
				selector: '.wm-button-container',
				comment: 'Applied to the button container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['containercomponent'],
		selectors: [
			{ selector: '.wm-container', comment: 'Container component', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['listcomponent'],
		selectors: [
			{ selector: '.wm-list', comment: 'List component', customCssKey: 'container' },
			{ selector: '.wm-list-pagination', comment: 'Pagination component' },
			{ selector: '.wm-list-pagination-buttons', comment: 'Pagination buttons component' }
		],
		variables: []
	},
	{
		components: ['drawercomponent'],
		selectors: [
			{ selector: '.wm-drawer', comment: 'main drawer element', customCssKey: 'drawer' },
			{ selector: '.wm-drawer-button', comment: 'button to open drawer', customCssKey: 'button' },
			{
				selector: '.wm-drawer-button-container',
				comment: 'container for button to open drawer',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['verticalsplitpanescomponent'],
		selectors: [
			{
				selector: '.wm-vertical-split-panes',
				comment: 'Vertical split panes component',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['horizontalsplitpanescomponent'],
		selectors: [
			{
				selector: '.wm-horizontal-split-panes',
				comment: 'Horizontal split panes component',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['modalcomponent'],
		selectors: [
			{ selector: '.wm-modal', comment: 'main modal element', customCssKey: 'popup' },
			{ selector: '.wm-modal-button', comment: 'button to open modal', customCssKey: 'button' },
			{
				selector: '.wm-modal-button-container',
				comment: 'container for button to open modal',
				customCssKey: 'buttonContainer'
			}
		],
		variables: []
	},
	{
		components: ['rangecomponent', 'slidercomponent'],
		selectors: [
			{ selector: '.wm-slider-bar', comment: 'Slider bar', customCssKey: 'bar' },
			{ selector: '.wm-slider-handle', comment: 'Slider handle', customCssKey: 'handle' },
			{ selector: '.wm-slider-limits', comment: 'Slider limits', customCssKey: 'limits' },
			{ selector: '.wm-slider-value', comment: 'Slider value', customCssKey: 'value' },

			{ selector: '.rangeSlider', comment: 'main slider element' },
			{ selector: '.rangeSlider.vertical', comment: 'if slider is vertical' },
			{ selector: '.rangeSlider.focus', comment: 'if slider is focussed' },
			{ selector: '.rangeSlider.range', comment: 'if slider is a range' },
			{ selector: '.rangeSlider.min', comment: 'if slider is a min-range' },
			{ selector: '.rangeSlider.max', comment: 'if slider is a max-range' },
			{ selector: '.rangeSlider.pips', comment: 'if slider has visible pips' },
			{ selector: '.rangeSlider.pip-labels', comment: 'if slider has labels for pips' },
			{
				selector: '.rangeSlider > .rangeHandle',
				comment: 'positioned wrapper for the handle/float'
			},
			{
				selector: '.rangeSlider > .rangeHandle.active',
				comment: 'if a handle is active in any way'
			},
			{
				selector: '.rangeSlider > .rangeHandle.press',
				comment: 'if a handle is being pressed down'
			},
			{
				selector: '.rangeSlider > .rangeHandle.hoverable',
				comment: 'if the handles allow hover effect'
			},
			{
				selector: '.rangeSlider > .rangeHandle > .rangeNub',
				comment: 'the actual nub rendered as a handle'
			},
			{
				selector: '.rangeSlider > .rangeHandle > .rangeFloat',
				comment: 'the floating value above the handle'
			},
			{ selector: '.rangeSlider > .rangeBar', comment: 'the range between the two handles' },
			{ selector: '.rangeSlider > .rangePips', comment: 'the container element for the pips' },
			{ selector: '.rangeSlider > .rangePips.focus', comment: 'if slider is focussed' },
			{ selector: '.rangeSlider > .rangePips.vertical', comment: 'if slider is vertical' },
			{ selector: '.rangeSlider > .rangePips > .pip', comment: 'an individual pip' },
			{
				selector: '.rangeSlider > .rangePips > .pip.first',
				comment: 'the first pip on the slider'
			},
			{ selector: '.rangeSlider > .rangePips > .pip.last', comment: 'the last pip on the slider' },
			{ selector: '.rangeSlider > .rangePips > .pip.selected', comment: 'if a pip is selected' },
			{
				selector: '.rangeSlider > .rangePips > .pip.in-range',
				comment: 'if a pip is somewhere in the range'
			},
			{ selector: '.rangeSlider > .rangePips > .pip > .pipVal', comment: 'the label for the pip' }
		],
		variables: [
			{ variable: '--range-slider', value: '#d7dada', comment: 'slider main background color' },
			{ variable: '--range-handle-inactive', value: '#99a2a2', comment: 'inactive handle color' },
			{ variable: '--range-handle', value: '#838de7', comment: 'non-focussed handle color' },
			{ variable: '--range-handle-focus', value: '#4a40d4', comment: 'focussed handle color' },
			{ variable: '--range-handle-border', value: 'var(--range-handle)' },
			{
				variable: '--range-range-inactive',
				value: 'var(--range-handle-inactive)',
				comment: 'inactive range bar background color'
			},
			{
				variable: '--range-range',
				value: 'var(--range-handle-focus)',
				comment: 'active range bar background color'
			},
			{
				variable: '--range-float-inactive',
				value: 'var(--range-handle-inactive)',
				comment: 'inactive floating label background color'
			},
			{
				variable: '--range-float',
				value: 'var(--range-handle-focus)',
				comment: 'floating label background color'
			},
			{ variable: '--range-float-text', value: 'white', comment: 'text color on floating label' }
		],
		root: '.rangeSlider',
		link: 'https://simeydotme.github.io/svelte-range-slider-pips/#styling'
	},
	{
		components: ['tabscomponent'],
		selectors: [
			{ selector: '.wm-tabs-container', comment: 'Tabs container', customCssKey: 'container' },
			{ selector: '.wm-tabs-tabRow', comment: 'Tabs row', customCssKey: 'tabRow' },
			{ selector: '.wm-tabs-alltabs', comment: 'All tabs', customCssKey: 'allTabs' },
			{ selector: '.wm-tabs-selectedTab', comment: 'Selected tab', customCssKey: 'selectedTab' }
		],
		variables: []
	},
	{
		components: ['carousellistcomponent'],
		selectors: [
			{ selector: '.wm-carousel', comment: 'Carousel component', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['formcomponent'],
		selectors: [
			{ selector: '.wm-submit', comment: 'Submit component', customCssKey: 'container' },
			{ selector: '.wm-submit-button', comment: 'Submit button', customCssKey: 'button' }
		],
		variables: []
	},
	{
		components: ['numberinputcomponent'],
		selectors: [
			{ selector: '.wm-number-input', comment: 'Number component', customCssKey: 'input' }
		],
		variables: []
	},
	{
		components: ['currencycomponent'],
		selectors: [
			{ selector: '.wm-currency-input', comment: 'Currency component', customCssKey: 'input' }
		],
		variables: []
	},
	{
		components: ['dateinputcomponent'],
		selectors: [{ selector: '.wm-date-input', comment: 'Date component', customCssKey: 'input' }],
		variables: []
	},
	{
		components: [
			'emailinputcomponent',
			'textinputcomponent',
			'textareainputcomponent',
			'passwordinputcomponent'
		],
		selectors: [{ selector: '.wm-text-input', comment: 'Text component', customCssKey: 'input' }],
		variables: []
	},
	{
		components: ['htmlcomponent'],
		selectors: [{ selector: '.wm-html', comment: 'HTML component', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['tablecomponent'],
		selectors: [
			{ selector: '.wm-table-container', comment: 'Table component', customCssKey: 'container' },
			{ selector: '.wm-table-header', comment: 'Table header', customCssKey: 'tableHeader' },
			{ selector: '.wm-table-body', comment: 'Table body', customCssKey: 'tableBody' },
			{ selector: '.wm-table-footer', comment: 'Table footer', customCssKey: 'tableFooter' },
			{ selector: '.wm-table-row-selected', comment: 'Selected row' },
			{ selector: '.wm-table-row', comment: 'Table row' }
		],
		variables: []
	},
	{
		components: ['aggridcomponent', 'aggridcomponentee'],
		selectors: [
			{ selector: '.wm-aggrid-container', comment: 'Ag grid container', customCssKey: 'container' }
		],
		variables: [
			{
				variable: '--ag-alpine-active-color',
				value: '#4a40d4',
				comment:
					'Accent colour used for checked checkboxes, range selections, row hover, row selections, selected tab underlines, and input focus outlines in the Alpine theme'
			},
			{
				variable: '--ag-foreground-color',
				value: '#ffffff',
				comment: 'Colour of text and icons in primary UI elements like menus'
			},
			{
				variable: '--ag-background-color',
				value: '#1e1f20',
				comment: 'Background colour of the grid'
			}
		],
		link: 'https://www.ag-grid.com/javascript-data-grid/global-style-customisation-variables/',
		root: '.ag-theme-alpine'
	},
	{
		components: ['steppercomponent'],
		selectors: [
			{ selector: 'wm-stepper', comment: 'Stepper component', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['fileinputcomponent'],
		selectors: [
			{ selector: '.wm-file-input', comment: 'File input component', customCssKey: 'input' }
		],
		variables: []
	},
	{
		components: ['checkboxcomponent'],
		selectors: [
			{ selector: '.wm-toggle-text', comment: 'Checkbox component label', customCssKey: 'text' },
			{
				selector: '.wm-toggle-container',
				comment: 'Checkbox component container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['imagecomponent'],
		selectors: [{ selector: '.wm-image', comment: 'Image component', customCssKey: 'image' }],
		variables: []
	},
	{
		components: ['pdfcomponent'],
		selectors: [{ selector: '.wm-pdf', comment: 'PDF component', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['horizontaldividercomponent'],
		selectors: [
			{
				selector: '.wm-horizontal-divider',
				comment: 'Horizontal divider component',
				customCssKey: 'divider'
			},
			{
				selector: '.wm-horizontal-divider-container',
				comment: 'Horizontal divider container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['verticaldividercomponent'],
		selectors: [
			{
				selector: '.wm-vertical-divider',
				comment: 'Vertical divider component',
				customCssKey: 'divider'
			},
			{
				selector: '.wm-vertical-divider-container',
				comment: 'Vertical divider container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['logcomponent', 'jobidlogcomponent'],
		selectors: [
			{ selector: '.wm-log-header', comment: 'Log header', customCssKey: 'header' },
			{ selector: '.wm-log-container', comment: 'Log container', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['mapcomponent'],
		selectors: [{ selector: '.wm-map', comment: 'Map component', customCssKey: 'map' }],
		variables: []
	},
	{
		components: ['iconcomponent'],
		selectors: [
			{ selector: '.wm-icon', comment: 'Icon component', customCssKey: 'icon' },
			{ selector: '.wm-icon-container', comment: 'Icon container', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['flowstatuscomponent', 'jobidflowstatuscomponent'],
		selectors: [
			{ selector: '.wm-flow-status-header', comment: 'Flow status header', customCssKey: 'header' },
			{
				selector: '.wm-flow-status-container',
				comment: 'Flow status container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['selecttabcomponent'],
		selectors: [
			{ selector: '.wm-select-tab-row', comment: 'Select tab row', customCssKey: 'tabRow' },
			{ selector: '.wm-select-tab', comment: 'Select tab', customCssKey: 'tab' },
			{
				selector: '.wm-select-tab-selected',
				comment: 'Select tab selected',
				customCssKey: 'selectedTab'
			}
		],
		variables: []
	},
	{
		components: ['piechartcomponent'],
		selectors: [{ selector: '.wm-pie-chart', comment: 'Pie chart', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['quillcomponent'],
		selectors: [
			{ selector: '.ql-toolbar', comment: 'Quill toolbar' },
			{ selector: '.ql-stroke', comment: 'Quill stroke' },
			{ selector: '.ql-fill', comment: 'Quill fill' },
			{ selector: '.ql-container', comment: 'Quill container' }
		],
		variables: []
	},
	{
		components: ['formbuttoncomponent'],
		selectors: [
			{ selector: '.wm-modal-form-popup', comment: 'Modal form popup', customCssKey: 'popup' },
			{ selector: '.wm-modal-form-button', comment: 'Modal form button', customCssKey: 'button' }
		],
		variables: []
	},
	{
		components: ['downloadcomponent'],
		selectors: [
			{ selector: '.wm-download-button', comment: 'Download button', customCssKey: 'button' },
			{ selector: '.wm-download-button-container', comment: 'Download button container' }
		],
		variables: []
	},
	{
		components: ['barchartcomponent'],
		selectors: [{ selector: '.wm-bar-chart', comment: 'Bar chart', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['scatterchartcomponent'],
		selectors: [
			{ selector: '.wm-scatter-chart', comment: 'Scatter chart', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['textcomponent'],
		selectors: [
			{ selector: '.wm-text', comment: 'Text component', customCssKey: 'text' },
			{ selector: '.wm-text-container', comment: 'Text container', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['chartjscomponent'],
		selectors: [{ selector: '.wm-chartjs', comment: 'ChartJS', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['timeseriescomponent'],
		selectors: [{ selector: '.wm-timeseries', comment: 'Time series', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['conditionalwrapper'],
		selectors: [
			{ selector: '.wm-conditional-tabs', comment: 'Conditional tabs', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['displaycomponent'],
		selectors: [
			{ selector: '.wm-rich-result-header', comment: 'Rich result header', customCssKey: 'header' },
			{
				selector: '.wm-rich-result-container',
				comment: 'Rich result container',
				customCssKey: 'container'
			}
		],
		variables: []
	},
	{
		components: ['mardowncomponent'],
		selectors: [
			{ selector: '.wm-markdown', comment: 'Markdown component', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['schemaformcomponent'],
		selectors: [
			{ selector: '.wm-schema-form', comment: 'Schema form component', customCssKey: 'container' }
		],
		variables: []
	},
	{
		components: ['selectstepcomponent'],
		selectors: [{ selector: '.wm-select-step', comment: 'Select step', customCssKey: 'container' }],
		variables: []
	},
	{
		components: ['selectcomponent', 'resourceselectcomponent'],
		selectors: [{ selector: '.svelte-select', comment: 'Svelte select', customCssKey: 'input' }],
		variables: []
	},

	{
		components: ['multiselectcomponent'],
		selectors: [
			{
				selector: '.multiselect',
				comment: 'top-level wrapper div'
			},
			{
				selector: 'multiselect.open',
				comment: 'top-level wrapper div when dropdown open'
			},
			{
				selector: '.multiselect.disabled',
				comment: 'top-level wrapper div when in disabled state'
			},
			{
				selector: '.multiselect > ul.selected',
				comment: 'selected list'
			},
			{
				selector: '.multiselect > ul.selected > li',
				comment: 'selected list items'
			},
			{
				selector: '.multiselect button',
				comment: 'target all buttons in this component'
			},
			{
				selector: '.multiselect > ul.selected > li button, button.remove-all',
				comment: 'buttons to remove a single or all selected options at once'
			},
			{
				selector: '.multiselect > input[autocomplete]',
				comment: 'input inside the top-level wrapper div'
			},
			{
				selector: '.multiselect > ul.options',
				comment: 'dropdown options'
			},
			{
				selector: '.multiselect > ul.options > li',
				comment: 'dropdown list items'
			},
			{
				selector: '.multiselect > ul.options > li.selected',
				comment: 'selected options in the dropdown list'
			},
			{
				selector: '.multiselect > ul.options > li:not(.selected):hover',
				comment: 'unselected but hovered options in the dropdown list'
			},
			{
				selector: '.multiselect > ul.options > li.active',
				comment:
					'active item, navigated to with up/down arrow keys and ready to be selected by pressing enter'
			},
			{
				selector: '.multiselect > ul.options > li.disabled',
				comment: 'options with disabled key set to true'
			}
		],
		variables: [
			{
				variable: '--sms-border',
				value: '1pt solid lightgray',
				comment:
					'Change this to e.g. to 1px solid red to indicate this form field is in an invalid state.'
			},
			{ variable: '--sms-border-radius', value: '3pt' },
			{ variable: '--sms-padding', value: '0 3pt' },
			{ variable: '--sms-bg', value: '' },
			{ variable: '--sms-text-color', value: '' },
			{ variable: '--sms-min-height', value: '22pt' },
			{ variable: '--sms-width', value: '' },
			{ variable: '--sms-max-width', value: '' },
			{ variable: '--sms-margin', value: '' },
			{ variable: '--sms-font-size', value: 'inherit' },
			{
				variable: '--sms-open-z-index',
				value: '4',
				comment:
					'Increase this if needed to ensure the dropdown list is displayed atop all other page elements.'
			},
			{
				variable: '--sms-focus-border',
				value: '1pt solid var(--sms-active-color, cornflowerblue)',
				comment:
					'Border when component has focus. Defaults to --sms-active-color which in turn defaults to cornflowerblue.'
			},
			{
				variable: '--sms-disabled-bg',
				value: 'lightgray',
				comment: 'Background when in disabled state.'
			},
			{ variable: '--sms-placeholder-color', value: '' },
			{ variable: '--sms-placeholder-opacity', value: '' },
			{
				variable: '--sms-selected-bg',
				value: 'rgba(0, 0, 0, 0.15)',
				comment: 'Background of selected options.'
			},
			{
				variable: '--sms-selected-li-padding',
				value: '1pt 5pt',
				comment: 'Height of selected options.'
			},
			{
				variable: '--sms-selected-text-color',
				value: 'var(--sms-text-color)',
				comment: 'Text color for selected options.'
			},
			{
				variable: '--sms-remove-btn-hover-color',
				value: 'lightskyblue',
				comment:
					'Color of the remove-icon buttons for removing all or individual selected options when in :focus or :hover state.'
			},
			{
				variable: '--sms-remove-btn-hover-bg',
				value: 'rgba(0, 0, 0, 0.2)',
				comment: 'Background for hovered remove buttons.'
			},
			{ variable: '--sms-options-bg', value: 'white', comment: 'Background of dropdown list.' },
			{
				variable: '--sms-options-max-height',
				value: '50vh',
				comment: 'Maximum height of options dropdown.'
			},
			{
				variable: '--sms-options-overscroll',
				value: 'none',
				comment:
					'Whether scroll events bubble to parent elements when reaching the top/bottom of the options dropdown. See MDN.'
			},
			{
				variable: '--sms-options-shadow',
				value: '0 0 14pt -8pt black',
				comment: 'Box shadow of dropdown list.'
			},
			{ variable: '--sms-options-border', value: '' },
			{ variable: '--sms-options-border-width', value: '' },
			{ variable: '--sms-options-border-radius', value: '1ex' },
			{ variable: '--sms-options-padding', value: '' },
			{ variable: '--sms-options-margin', value: 'inherit' },
			{
				variable: '--sms-options-scroll-margin',
				value: '100px',
				comment:
					'Top/bottom margin to keep between dropdown list items and top/bottom screen edge when auto-scrolling list to keep items in view.'
			},
			{
				variable: '--sms-li-selected-bg',
				comment: 'Background of selected list items in options pane.',
				value: ''
			},
			{
				variable: '--sms-li-selected-color',
				comment: 'Text color of selected list items in options pane.',
				value: ''
			},
			{
				variable: '--sms-li-active-bg',
				value: 'var(--sms-active-color, rgba(0, 0, 0, 0.15))',
				comment:
					'Background of active options. Options in the dropdown list become active either by mouseover or by navigating to them with arrow keys. Selected options become active when selectedOptionsDraggable=true and an option is being dragged to a new position. Note the active option in that case is not the dragged option but the option under it whose place it will take on drag end.'
			},
			{
				variable: '--sms-li-disabled-bg',
				value: '#f5f5f6',
				comment: 'Background of disabled options in the dropdown list.'
			},
			{
				variable: '--sms-li-disabled-text',
				value: '#b8b8b8',
				comment: 'Text color of disabled option in the dropdown list.'
			}
		],
		root: '.multiselect'
	}
]

export const allClasses = customisationByComponent
	.map((c) => c.selectors.map((x) => x.selector))
	.flat()

export function hasStyleValue(obj: ComponentCssProperty | undefined) {
	if (!obj) return false

	return obj.style !== ''
}

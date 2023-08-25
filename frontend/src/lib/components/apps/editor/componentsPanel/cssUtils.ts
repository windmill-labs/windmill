import * as csstree from 'css-tree'

export function sanitizeCss(css: string, authorizedClassNames: string[]) {
	const ast = csstree.parse(css)
	const removedClassNames: string[] = []

	csstree.walk(ast, (node: any, item, list) => {
		if (node.type === 'Rule') {
			let shouldRemoveRule = true

			csstree.walk(node, (innerNode: any) => {
				if (innerNode.type === 'ClassSelector' && authorizedClassNames.includes(innerNode.name)) {
					shouldRemoveRule = false
				}
				if (shouldRemoveRule && innerNode.name) {
					removedClassNames.push(innerNode.name)
				}
			})

			if (shouldRemoveRule) {
				list.remove(item)
			}
		}
	})

	return {
		css: csstree.generate(ast),
		removedClassNames
	}
}

export const authorizedClassnames = [
	'wm-container', // Migrated DOC
	'wm-list', // Migrated DOC
	'wm-list-pagination', // Migrated DOC
	'wm-list-pagination-buttons', // Migrated DOC
	'wm-drawer', // Migrated DOC
	'wm-drawer-button', // Migrated DOC
	'wm-drawer-button-container', // Migrated DOC
	'wm-button', // Migrated DOC
	'wm-button-wrapper', // Migrated DOC
	'wm-button-container', // Migrated DOC
	'wm-vertical-split-panes', // Migrated DOC
	'wm-horizontal-split-panes', // Migrated DOC
	'wm-modal', // Migrated DOC
	'wm-modal-button', // Migrated DOC
	'wm-modal-button-container', // Migrated DOC
	'wm-tabs-container', // Migrated DOC
	'wm-tabs-tabRow', // Migrated DOC
	'wm-tabs-alltabs', // Migrated DOC
	'wm-tabs-selectedTab', // Migrated DOC
	'wm-carousel', // Migrated DOC
	'wm-submit', // Migrated DOC
	'wm-submit-button', // Migrated DOC
	'wm-number-input', // Migrated DOC
	'wm-currency-input', // Migrated DOC
	'wm-date-input', // Migrated DOC
	'wm-text-input', // Migrated DOC
	'wm-html', // Migrated DOC
	'wm-table-container', // Migrated DOC
	'wm-table-header', // Migrated DOC
	'wm-table-body', // Migrated DOC
	'wm-table-footer', // Migrated DOC
	'wm-table-row-selected', // Migrated DOC
	'wm-table-row', // Migrated DOC
	'wm-stepper', // Migrated DOC
	'wm-file-input', // Migrated DOC
	'wm-toggle', // Migrated DOC
	'wm-image', // Migrated DOC
	'wm-pdf', // Migrated DOC
	'wm-horizontal-divider', // Migrated DOC
	'wm-vertical-divider', // Migrated DOC
	'wm-horizontal-divider-container', // Migrated DOC
	'wm-vertical-divider-container', // Migrated DOC
	'wm-log-header', // Migrated DOC
	'wm-log-container', // Migrated DOC
	'wm-map', // Migrated DOC
	'wm-icon', // Migrated DOC
	'wm-icon-container', // Migrated DOC
	'wm-flow-status-header', // Migrated DOC
	'wm-flow-status-container', // Migrated DOC
	'wm-select-tab-row', // Migrated DOC
	'wm-select-tab', // Migrated DOC
	'wm-select-tab-selected', // Migrated DOC

	'ql-toolbar', // Migrated
	'ql-stroke', // Migrated
	'ql-fill', // Migrated
	'ql-container', // Migrated
	/**
	 *
	 *
	 *
	 *
	 *
	 *
	 *
	 */

	'wm-conditional-tabs', // Migrated
	'wm-sidebar-tabs',
	'wm-invisible-tabs',
	'wm-modal-form',
	'wm-download-button',
	'wm-form',
	'wm-select',
	'wm-resource-select',
	'wm-multiselect',
	'wm-select-step',
	'wm-aggrid-table',
	'wm-text',
	'wm-rich-result',
	'wm-bar-line-chart',
	'wm-pie-chart',
	'wm-vega-lite',
	'wm-plotly',
	'wm-scatter-chart',
	'wm-timeseries',
	'wm-chartjs'
]

interface Selector {
	selector: string
	comment?: string | undefined
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
}

export const customisationByComponent: Customisation[] = [
	{
		components: ['app'],
		selectors: [
			{ selector: '.wm-app', comment: 'main app element' },
			{ selector: '.wm-app-viewer', comment: 'main app viewer element' },
			{ selector: '.wm-app-grid', comment: 'main app grid element' },
			{ selector: '.wm-app-component', comment: 'main app component element' }
		],
		variables: []
	},
	{
		components: ['buttoncomponent'],
		selectors: [
			{ selector: '.wm-button', comment: 'main button element' },
			{ selector: '.wm-button-wrapper', comment: 'wrapper for button' },
			{ selector: '.wm-button-container', comment: 'container for button' }
		],
		variables: []
	},
	{
		components: ['containercomponent'],
		selectors: [{ selector: '.wm-container', comment: 'Container component' }],
		variables: []
	},
	{
		components: ['listcomponent'],
		selectors: [
			{ selector: '.wm-list', comment: 'List component' },
			{ selector: '.wm-list-pagination', comment: 'Pagination component' },
			{ selector: '.wm-list-pagination-buttons', comment: 'Pagination buttons component' }
		],
		variables: []
	},
	{
		components: ['drawercomponent'],
		selectors: [
			{ selector: '.wm-drawer', comment: 'main drawer element' },
			{ selector: '.wm-drawer-button', comment: 'button to open drawer' },
			{ selector: '.wm-drawer-button-container', comment: 'container for button to open drawer' }
		],
		variables: []
	},
	{
		components: ['verticalsplitpanescomponent'],
		selectors: [
			{ selector: '.wm-vertical-split-panes', comment: 'Vertical split panes component' }
		],
		variables: []
	},
	{
		components: ['horizontalsplitpanescomponent'],
		selectors: [
			{ selector: '.wm-horizontal-split-panes', comment: 'Horizontal split panes component' }
		],
		variables: []
	},
	{
		components: ['modalcomponent'],
		selectors: [
			{ selector: '.wm-modal', comment: 'main modal element' },
			{ selector: '.wm-modal-button', comment: 'button to open modal' },
			{ selector: '.wm-modal-button-container', comment: 'container for button to open modal' }
		],
		variables: []
	},
	{
		components: ['rangecomponent', 'slidercomponent'],
		selectors: [
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
		link: 'https://simeydotme.github.io/svelte-range-slider-pips/#styling'
	},
	{
		components: ['tabscomponent'],
		selectors: [
			{ selector: '.wm-tabs-container', comment: 'Tabs container' },
			{ selector: '.wm-tabs-tabRow', comment: 'Tabs row' },
			{ selector: '.wm-tabs-alltabs', comment: 'All tabs' },
			{ selector: '.wm-tabs-selectedTab', comment: 'Selected tab' }
		],
		variables: []
	},
	{
		components: ['carousellistcomponent'],
		selectors: [{ selector: '.wm-carousel', comment: 'Carousel component' }],
		variables: []
	},
	{
		components: ['submitcomponent'],
		selectors: [
			{ selector: '.wm-submit', comment: 'Submit component' },
			{ selector: '.wm-submit-button', comment: 'Submit button' }
		],
		variables: []
	},
	{
		components: ['numbercomponent'],
		selectors: [{ selector: '.wm-number-input', comment: 'Number component' }],
		variables: []
	},
	{
		components: ['currencycomponent'],
		selectors: [{ selector: '.wm-currency-input', comment: 'Currency component' }],
		variables: []
	},
	{
		components: ['datecomponent'],
		selectors: [{ selector: '.wm-date-input', comment: 'Date component' }],
		variables: []
	},
	{
		components: ['textcomponent'],
		selectors: [{ selector: '.wm-text-input', comment: 'Text component' }],
		variables: []
	},
	{
		components: ['htmlcomponent'],
		selectors: [{ selector: '.wm-html', comment: 'HTML component' }],
		variables: []
	},
	{
		components: ['tablecomponent'],
		selectors: [
			{ selector: '.wm-table-container', comment: 'Table component' },
			{ selector: '.wm-table-header', comment: 'Table header' },
			{ selector: '.wm-table-body', comment: 'Table body' },
			{ selector: '.wm-table-footer', comment: 'Table footer' },
			{ selector: '.wm-table-row-selected', comment: 'Selected row' },
			{ selector: '.wm-table-row', comment: 'Table row' }
		],
		variables: []
	},
	{
		components: ['steppercomponent'],
		selectors: [{ selector: 'wm-stepper', comment: 'Stepper component' }],
		variables: []
	},
	{
		components: ['fileinputcomponent'],
		selectors: [{ selector: 'wm-file-input', comment: 'File input component' }],
		variables: []
	},
	{
		components: ['checkboxcomponent'],
		selectors: [{ selector: 'wm-toggle', comment: 'Checkbox component' }],
		variables: []
	},
	{
		components: ['imagecomponent'],
		selectors: [{ selector: '.wm-image', comment: 'Image component' }],
		variables: []
	},
	{
		components: ['pdfcomponent'],
		selectors: [{ selector: '.wm-pdf', comment: 'PDF component' }],
		variables: []
	},
	{
		components: ['dividercomponent'],
		selectors: [
			{ selector: '.wm-horizontal-divider', comment: 'Horizontal divider component' },
			{ selector: '.wm-vertical-divider', comment: 'Vertical divider component' },
			{ selector: '.wm-horizontal-divider-container', comment: 'Horizontal divider container' },
			{ selector: '.wm-vertical-divider-container', comment: 'Vertical divider container' }
		],
		variables: []
	},
	// log
	{
		components: ['logcomponent'],
		selectors: [
			{ selector: '.wm-log-header', comment: 'Log header' },
			{ selector: '.wm-log-container', comment: 'Log container' }
		],
		variables: []
	},
	{
		components: ['mapcomponent'],
		selectors: [{ selector: '.wm-map', comment: 'Map component' }],
		variables: []
	},
	{
		components: ['iconcomponent'],
		selectors: [
			{ selector: '.wm-icon', comment: 'Icon component' },
			{ selector: '.wm-icon-container', comment: 'Icon container' }
		],
		variables: []
	},
	{
		components: ['flowstatuscomponent'],
		selectors: [
			{ selector: '.wm-flow-status-header', comment: 'Flow status header' },
			{ selector: '.wm-flow-status-container', comment: 'Flow status container' }
		],
		variables: []
	},
	{
		components: ['selecttabcomponent'],
		selectors: [
			{ selector: '.wm-select-tab-row', comment: 'Select tab row' },
			{ selector: '.wm-select-tab', comment: 'Select tab' },
			{ selector: '.wm-select-tab-selected', comment: 'Select tab selected' }
		],
		variables: []
	}
]

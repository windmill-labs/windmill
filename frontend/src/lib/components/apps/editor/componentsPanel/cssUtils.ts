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
	'wm-container', // Migrated
	// List
	'wm-list', // Migrated
	'wm-list-pagination', // Migrated
	'wm-list-pagination-buttons', // Migrated

	'wm-divider-x',
	'wm-divider-y',

	'wm-drawer', // Migrated
	'wm-drawer-button', // Migrated
	'wm-drawer-button-container', // Migrated

	'wm-vertical-split-panes', // Migrated
	'wm-horizontal-split-panes', // Migrated

	'wm-modal', // Migrated
	'wm-modal-button', // Migrated
	'wm-modal-button-container', // Migrated

	'wm-stepper', // Migrated

	'wm-tabs-container', // Migrated
	'wm-tabs-tabRow', // Migrated
	'wm-tabs-alltabs', // Migrated
	'wm-tabs-selectedTab', // Migrated

	'wm-conditional-tabs', // Migrated
	'wm-sidebar-tabs',
	'wm-invisible-tabs',

	'wm-button', // Migrated
	'wm-button-wrapper', // Migrated
	'wm-button-container', // Migrated

	'wm-submit', // Migrated
	'wm-submit-button', // Migrated

	'wm-modal-form',
	'wm-download-button',

	'wm-form',
	'wm-text-input', // Migrated

	// Quill
	'ql-toolbar', // Migrated
	'ql-stroke', // Migrated
	'ql-fill', // Migrated
	'ql-container', // Migrated

	'wm-number', // Migrated
	'wm-currency', // Migrated
	'wm-date',
	'wm-file-input',
	'wm-toggle',
	'wm-select',
	'wm-resource-select',
	'wm-multiselect',
	'wm-select-tab',
	'wm-select-step',

	'wm-table-container', // Migrated
	'wm-table-header', // Migrated
	'wm-table-body', // Migrated
	'wm-table-footer', // Migrated
	'wm-table-row-selected', // Migrated
	'wm-table-row', // Migrated

	'wm-aggrid-table',

	'wm-text',
	'wm-icon',
	'wm-image',
	'wm-map',
	'wm-html',
	'wm-pdf',
	'wm-rich-result',
	'wm-log',
	'wm-flow-status',

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
	link: string
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
		variables: [],
		link: ''
	},

	// Drawer
	{
		components: ['drawercomponent'],
		selectors: [
			{ selector: '.wm-drawer', comment: 'main drawer element' },
			{ selector: '.wm-drawer-button', comment: 'button to open drawer' },
			{ selector: '.wm-drawer-button-container', comment: 'container for button to open drawer' }
		],
		variables: [],
		link: ''
	},

	// Ranges and Sliders
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
	}
]

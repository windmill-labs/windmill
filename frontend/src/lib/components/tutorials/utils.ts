import type { FlowModule, OpenFlow } from '$lib/gen'
import { deepEqual } from 'fast-equals'
import { findGridItem } from '../apps/editor/appUtils'
import type { App } from '../apps/types'

export function setInputBySelector(selector: string, value: string) {
	const input = document.querySelector(selector) as HTMLInputElement

	if (input) {
		input.value = value
		input.dispatchEvent(new Event('input', { bubbles: true }))
	}
}

export function clickButtonBySelector(selector: string) {
	const button = document.querySelector(selector) as HTMLButtonElement

	if (button) {
		button.click()
	}
}

export function clickFirstButtonBySelector(selector: string) {
	const buttons = document.querySelector(selector)
	const button = buttons?.childNodes[0] as HTMLButtonElement

	if (button) {
		button.click()
	}
}

export function triggerPointerDown(selector: string) {
	const elem = document.querySelector(selector) as HTMLElement

	if (elem) {
		elem.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
	}
}

export function selectOptionsBySelector(selector: string, value: string) {
	const select = document.querySelector(selector) as HTMLSelectElement

	if (select) {
		select.value = value
		select.dispatchEvent(new Event('change', { bubbles: true }))
	}
}

export function isFlowTainted(flow: OpenFlow) {
	return (
		flow.value.modules.length > 0 || Object.keys((flow?.schema?.properties as any) ?? {}).length > 0
	)
}

const emptyApp = {
	grid: [
		{
			'3': {
				fixed: false,
				x: 0,
				y: 0,
				fullHeight: false,
				w: 6,
				h: 2
			},
			'12': {
				fixed: false,
				x: 0,
				y: 0,
				fullHeight: false,
				w: 12,
				h: 2
			},
			data: {
				type: 'containercomponent',
				configuration: {},
				customCss: {
					container: {
						class: '!p-0',
						style: ''
					}
				},
				numberOfSubgrids: 1,
				id: 'a'
			},
			id: 'a'
		}
	],
	fullscreen: false,
	unusedInlineScripts: [],
	hiddenInlineScripts: [],
	theme: {
		type: 'path',
		path: 'f/app_themes/theme_0'
	},
	subgrids: {
		'a-0': [
			{
				'3': {
					fixed: false,
					x: 0,
					y: 0,
					fullHeight: false,
					w: 6,
					h: 1
				},
				'12': {
					fixed: false,
					x: 0,
					y: 0,
					fullHeight: false,
					w: 6,
					h: 1
				},
				data: {
					type: 'textcomponent',
					configuration: {
						style: {
							type: 'static',
							value: 'Body'
						},
						copyButton: {
							type: 'static',
							value: false
						},
						tooltip: {
							type: 'evalv2',
							value: '',
							fieldType: 'text',
							expr: '`Author: ${ctx.author}`',
							connections: [
								{
									componentId: 'ctx',
									id: 'author'
								}
							]
						},
						disableNoText: {
							type: 'static',
							value: true,
							fieldType: 'boolean'
						}
					},
					componentInput: {
						type: 'templatev2',
						fieldType: 'template',
						eval: '${ctx.summary}',
						connections: [
							{
								id: 'summary',
								componentId: 'ctx'
							}
						]
					},
					customCss: {
						text: {
							class: 'text-xl font-semibold whitespace-nowrap truncate',
							style: ''
						},
						container: {
							class: '',
							style: ''
						}
					},
					horizontalAlignment: 'left',
					verticalAlignment: 'center',
					id: 'b'
				},
				id: 'b'
			},
			{
				'3': {
					fixed: false,
					x: 0,
					y: 1,
					fullHeight: false,
					w: 3,
					h: 1
				},
				'12': {
					fixed: false,
					x: 6,
					y: 0,
					fullHeight: false,
					w: 6,
					h: 1
				},
				data: {
					type: 'recomputeallcomponent',
					configuration: {},
					menuItems: [],
					horizontalAlignment: 'right',
					verticalAlignment: 'center',
					id: 'c'
				},
				id: 'c'
			}
		]
	},
	hideLegacyTopBar: true,
	norefreshbar: false
}

export function isAppTainted(app: App) {
	if (app.hideLegacyTopBar === true) {
		// An empty app should have only have a topbar and no hidden inline scripts

		if (Array.isArray(app.hiddenInlineScripts) && app.hiddenInlineScripts?.length > 0) {
			return true
		}

		// New apps have only a single component which is the topbar
		if (Array.isArray(app.grid) && app.grid.length > 1) {
			return true
		}

		// Check if the current app is different from an empty app
		return !deepEqual(app, emptyApp)
	} else {
		// For older apps,
		return !(app.grid?.length === 0 && app.hiddenInlineScripts?.length === 0)
	}
}

export function updateFlowModuleById(
	flow: OpenFlow,
	id: string,
	callback: (module: FlowModule) => void
) {
	const dfs = (modules: FlowModule[]) => {
		for (const module of modules) {
			if (module.id === id) {
				callback(module)
				return
			}

			if (module.value.type === 'forloopflow') {
				dfs(module.value.modules)
			} else if (module.value.type === 'branchone') {
				module.value.branches.forEach((branch) => dfs(branch.modules))
			} else if (module.value.type === 'branchall') {
				module.value.branches.forEach((branch) => dfs(branch.modules))
			}
		}
	}

	dfs(flow.value.modules)

	flow = flow
}

export function updateBackgroundRunnableCode(app: App, index: number, newCode: string) {
	const script = app.hiddenInlineScripts[index]
	if (script.type === 'runnableByName' && script.inlineScript) {
		script.inlineScript.content = newCode
	}

	app = app
}

export function updateInlineRunnableCode(app: App, componentId: string, newCode: string) {
	const gridItem = findGridItem(app, componentId)

	if (gridItem?.data.componentInput?.type === 'runnable') {
		if (
			gridItem.data.componentInput.runnable?.type === 'runnableByName' &&
			gridItem.data.componentInput.runnable.inlineScript
		) {
			gridItem.data.componentInput.runnable.inlineScript.content = newCode
		}
	}

	app = app
}

export function connectComponentSourceToOutput(app: App, componentId: string, targetId: string) {
	const gridItem = findGridItem(app, componentId)

	if (gridItem) {
		gridItem.data.componentInput = {
			type: 'evalv2',
			fieldType: 'object',

			expr: `${targetId}.result`,
			connections: [
				{
					componentId: targetId,
					id: 'result'
				}
			]
		}
	}

	app = app
}

export function connectInlineRunnableInputToComponentOutput(
	app: App,
	sourceComponentId: string,
	sourceField: string,
	targetComponentId: string,
	targetField: string,
	fieldType: string = 'text'
) {
	const gridItem = findGridItem(app, sourceComponentId)

	if (gridItem?.data.componentInput?.type === 'runnable') {
		// @ts-ignore
		gridItem.data.componentInput.fields = {
			[sourceField]: {
				type: 'evalv2',
				expr: `${targetComponentId}.${targetField}`,
				fieldType: fieldType,
				connections: [
					{
						componentId: targetComponentId,
						id: targetField
					}
				]
			}
		}
	}
}

function elementExists(selector: string): boolean {
	return !!document.querySelector(selector)
}

export function waitForElementLoading(
	selector: string,
	callback: () => void,
	interval: number = 50,
	maxAttempts: number = 30
): void {
	let attempts = 0

	const checkExistence = setInterval(() => {
		if (elementExists(selector)) {
			clearInterval(checkExistence)
			callback()
		} else if (attempts >= maxAttempts) {
			clearInterval(checkExistence)
			console.error('Element not found after multiple attempts.')
		}
		attempts++
	}, interval)
}

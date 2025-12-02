import type { FlowModule, OpenFlow } from '$lib/gen'
import { deepEqual } from 'fast-equals'
import { emptyApp } from '../apps/editor/appUtils'
import type { App } from '../apps/types'
import { findGridItem } from '../apps/editor/appUtilsCore'
import { isRunnableByName } from '../apps/inputType'

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
		return !deepEqual(app, emptyApp())
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
			// AI agent tools are leaf nodes - no traversal needed
		}
	}

	dfs(flow.value.modules)
}

export function updateBackgroundRunnableCode(app: App, index: number, newCode: string) {
	const script = app.hiddenInlineScripts[index]
	if (isRunnableByName(script) && script.inlineScript) {
		script.inlineScript.content = newCode
	}
}

export function updateInlineRunnableCode(app: App, componentId: string, newCode: string) {
	const gridItem = findGridItem(app, componentId)
	if (gridItem?.data.componentInput?.type === 'runnable') {
		if (
			isRunnableByName(gridItem.data.componentInput.runnable) &&
			gridItem.data.componentInput.runnable.inlineScript
		) {
			gridItem.data.componentInput.runnable.inlineScript.content = newCode
		}
	}
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

import type { FlowModule, OpenFlow } from '$lib/gen'
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

export function triggerAddFlowStep(index: number) {
	const button = document.querySelector(`#flow-editor-add-step-${index}`) as HTMLButtonElement

	if (button) {
		button.parentElement?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
	}
}

export function selectFlowStepKind(index: number) {
	const button = document.querySelector(
		`#flow-editor-insert-module > div > button:nth-child(${index})`
	) as HTMLButtonElement

	if (button) {
		button?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
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
	return flow.value.modules.length > 0 || Object.keys(flow?.schema?.properties).length > 0
}

export function isAppTainted(app: App) {
	return !(app.grid.length === 0 && app.hiddenInlineScripts?.length === 0)
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

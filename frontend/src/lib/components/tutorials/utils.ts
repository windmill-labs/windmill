import type { FlowModule, OpenFlow } from '$lib/gen'
import { deepEqual } from 'fast-equals'
import { emptyApp } from '../apps/editor/appUtils'
import type { App } from '../apps/types'
import { findGridItem } from '../apps/editor/appUtilsCore'
import { isRunnableByName } from '../apps/inputType'
import { wait } from '$lib/utils'

// Tutorial animation delay constants
export const DELAY_SHORT = 100
export const DELAY_MEDIUM = 300
export const DELAY_LONG = 500
export const DELAY_ANIMATION = 1500
export const DELAY_ANIMATION_LONG = 2500
export const DELAY_TYPING = 50
export const DELAY_CODE_CHAR = 2
export const DELAY_CODE_NEWLINE = 5

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

// Helper function to move cursor to element (for continuous cursor movement in tutorials)
export async function moveCursorToElement(
	cursor: HTMLElement,
	element: HTMLElement,
	duration: number = DELAY_ANIMATION
): Promise<void> {
	const rect = element.getBoundingClientRect()
	cursor.style.transition = `all ${duration / 1000}s ease-in-out`
	cursor.style.left = `${rect.left + rect.width / 2}px`
	cursor.style.top = `${rect.top + rect.height / 2}px`
	await wait(duration)
}

// Helper function to create a fake cursor element for tutorial animations
export function createFakeCursor(): HTMLElement {
	const fakeCursor = document.createElement('div')
	fakeCursor.style.cssText = `
		position: fixed;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background-color: rgba(59, 130, 246, 0.8);
		border: 2px solid white;
		pointer-events: none;
		z-index: 10000;
		transition: all 1.5s ease-in-out;
	`
	document.body.appendChild(fakeCursor)
	return fakeCursor
}

// Constants for cursor animation
const CURSOR_START_OFFSET = -100
const CURSOR_CLICK_SCALE = 0.8

// Helper function to create and animate a fake cursor with start position
export async function createFakeCursorWithStart(
	startElement: HTMLElement | null,
	endElement: HTMLElement,
	transitionDuration: number = 1.5
): Promise<HTMLElement> {
	const fakeCursor = createFakeCursor()

	const endRect = endElement.getBoundingClientRect()
	let startX: number, startY: number

	if (startElement) {
		const startRect = startElement.getBoundingClientRect()
		startX = startRect.left + startRect.width / 2
		startY = startRect.top + startRect.height / 2
	} else {
		startX = endRect.left + CURSOR_START_OFFSET
		startY = endRect.top + endRect.height / 2
	}

	fakeCursor.style.left = `${startX}px`
	fakeCursor.style.top = `${startY}px`

	await wait(DELAY_SHORT)

	fakeCursor.style.left = `${endRect.left + endRect.width / 2}px`
	fakeCursor.style.top = `${endRect.top + endRect.height / 2}px`

	await wait(transitionDuration * 1000)

	return fakeCursor
}

// Helper function to animate a fake cursor click
export async function animateFakeCursorClick(
	element: HTMLElement,
	transitionDuration: number = 1.5,
	options?: { usePointerEvents?: boolean; startElement?: HTMLElement | null }
): Promise<void> {
	const fakeCursor = await createFakeCursorWithStart(
		options?.startElement ?? null,
		element,
		transitionDuration
	)
	await wait(DELAY_MEDIUM)

	// Animate click (shrink cursor briefly)
	fakeCursor.style.transform = `scale(${CURSOR_CLICK_SCALE})`
	await wait(DELAY_SHORT)
	fakeCursor.style.transform = 'scale(1)'
	await wait(DELAY_SHORT)

	// Trigger pointer events if needed (flow graph uses pointer events instead of click)
	if (options?.usePointerEvents) {
		element.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
		element.dispatchEvent(new PointerEvent('pointerup', { bubbles: true }))
	}

	// Click the element
	element.click()
	await wait(DELAY_SHORT)

	// Remove fake cursor
	fakeCursor.remove()
}

// Helper function to animate cursor to element and click (for reusing a cursor across multiple clicks)
export async function animateCursorToElementAndClick(
	cursor: HTMLElement,
	element: HTMLElement,
	startOffset: number = CURSOR_START_OFFSET
): Promise<void> {
	const rect = element.getBoundingClientRect()
	
	// Set initial position (off-screen to the left)
	cursor.style.left = `${rect.left + startOffset}px`
	cursor.style.top = `${rect.top + rect.height / 2}px`
	await wait(DELAY_SHORT)
	
	// Animate to target position
	cursor.style.left = `${rect.left + rect.width / 2}px`
	cursor.style.top = `${rect.top + rect.height / 2}px`
	await wait(DELAY_ANIMATION)
	await wait(DELAY_MEDIUM)
	
	// Click on the element
	element.click()
	await wait(DELAY_SHORT)
}

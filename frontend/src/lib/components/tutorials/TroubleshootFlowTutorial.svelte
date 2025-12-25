<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow } from '$lib/gen'
	import { wait, type StateStore } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { updateProgress } from '$lib/tutorialUtils'
	import { DELAY_SHORT, DELAY_MEDIUM, DELAY_LONG, createFakeCursor } from './utils'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	// Flags to track if steps are complete
	let stepComplete = $state<Record<number, boolean>>({
		1: false,
		2: false,
		3: false,
		4: false,
		5: false,
		6: false,
		7: false
	})

	// Constants for cursor animation
	const CURSOR_START_OFFSET = -100
	const CURSOR_CLICK_SCALE = 0.8

	// DOM Selectors
	const SELECTORS = {
		testFlowButton: '#flow-editor-test-flow',
		testFlowDrawer: '#flow-editor-test-flow-drawer',
		flowPreviewContent: '#flow-preview-content',
		stepB: '#b'
	} as const

	// Text constants
	const TEXT = {
		convertToFahrenheit: 'Convert to Fahrenheit'
	} as const

	// Helper function to check if step is complete
	function checkStepComplete(step: number): boolean {
		if (!stepComplete[step]) {
			sendUserToast('Please wait...', false, [], undefined, 3000)
			return false
		}
		return true
	}

	// Helper function to create and animate a fake cursor with start position
	async function createFakeCursorWithStart(
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

	// Helper function to get element by selector (handles both querySelector and getElementById)
	function getElementBySelector(selector: string): HTMLElement | null {
		// If selector starts with #, try getElementById first, then fallback to querySelector
		if (selector.startsWith('#')) {
			const id = selector.slice(1)
			return document.getElementById(id) || document.querySelector(selector)
		}
		return document.querySelector(selector) as HTMLElement | null
	}

	// Helper function to animate a fake cursor click
	async function animateFakeCursorClick(
		element: HTMLElement,
		transitionDuration: number = 1.5,
		options?: { usePointerEvents?: boolean }
	): Promise<void> {
		const fakeCursor = await createFakeCursorWithStart(null, element, transitionDuration)
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

	// Helper function to find close button in drawer
	function findCloseButton(drawer: HTMLElement): HTMLElement | null {
		return Array.from(drawer.querySelectorAll('button')).find(btn => {
			const svg = btn.querySelector('svg.lucide-x')
			return svg !== null
		}) as HTMLElement | null
	}

	// Helper function to find button by text
	function findButtonByText(container: HTMLElement, text: string): HTMLElement | null {
		const buttons = Array.from(container.querySelectorAll('button'))
		return buttons.find(btn => btn.textContent?.includes(text)) as HTMLElement | null
	}

	export async function runTutorial() {
		// Load the pre-built flow immediately when tutorial starts
		await initFlow(preBuiltFlow, flowStore as StateStore<Flow>, flowStateStore)
		await wait(DELAY_MEDIUM)

		// Set the celsius input to 25
		await wait(DELAY_SHORT)
		const celsiusInput = document.querySelector('input[type="number"]') as HTMLInputElement
		if (celsiusInput) {
			celsiusInput.value = '25'
			celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
		}

		tutorial?.runTutorial()
	}

	// Pre-built flow - same as the flow builder tutorial result
	const preBuiltFlow: Flow = {
		summary: 'Temperature Converter',
		description: 'Convert Celsius to Fahrenheit and categorize the temperature',
		value: {
			modules: [
				{
					id: 'a',
					value: {
						type: 'rawscript',
						content:
							'export async function main(celsius: number) {\n  // Validate that the temperature is within a reasonable range\n  if (celsius < -273.15) {\n    throw new Error("Temperature cannot be below absolute zero (-273.15°C)");\n  }\n  \n  if (celsius > 1000) {\n    throw new Error("Temperature seems unreasonably high. Please check your input.");\n  }\n  \n  return {\n    celsius: celsius,\n    isValid: true,\n    message: "Temperature is valid"\n  };\n}',
						language: 'bun',
						input_transforms: {
							celsius: {
								expr: 'flow_input.celsius',
								type: 'javascript'
							}
						}
					},
					summary: 'Validate temperature input'
				},
				{
					id: 'b',
					value: {
						type: 'rawscript',
						content:
							'export async function main(celsius: number) {\n  // Convert Celsius to Fahrenheit using the formula: F = (C × 9/5) + 32\n  const fahrenheit = (celsius * 9/5) + 32;\n  \n  return {\n    celsius: celsiu,\n    fahrenheit: Math.round(fahrenheit * 100) / 100 // Round to 2 decimal places\n  };\n}',
						language: 'bun',
						input_transforms: {
							celsius: {
								expr: 'results.a.celsius',
								type: 'javascript'
							}
						}
					},
					summary: 'Convert to Fahrenheit'
				},
				{
					id: 'c',
					value: {
						type: 'rawscript',
						content:
							'export async function main(celsius: number, fahrenheit: number) {\n  // Categorize the temperature based on Celsius value\n  let category: string;\n  let emoji: string;\n  \n  if (celsius < 0) {\n    category = "Freezing";\n    emoji = "❄️";\n  } else if (celsius < 10) {\n    category = "Cold";\n    emoji = "🥶";\n  } else if (celsius < 20) {\n    category = "Cool";\n    emoji = "😊";\n  } else if (celsius < 30) {\n    category = "Warm";\n    emoji = "☀️";\n  } else {\n    category = "Hot";\n    emoji = "🔥";\n  }\n  \n  return {\n    celsius: celsius,\n    fahrenheit: fahrenheit,\n    category: category,\n    emoji: emoji\n  };\n}',
						language: 'bun',
						input_transforms: {
							celsius: {
								expr: 'results.b.celsius',
								type: 'javascript'
							},
							fahrenheit: {
								expr: 'results.b.fahrenheit',
								type: 'javascript'
							}
						}
					},
					summary: 'Categorize temperature'
				}
			]
		},
		schema: {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			type: 'object',
			properties: {
				celsius: {
					type: 'number',
					description: 'Temperature in Celsius',
					default: 25
				}
			},
			required: ['celsius'],
			order: ['celsius']
		},
		path: '',
		edited_at: '',
		edited_by: '',
		archived: false,
		extra_perms: {}
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={index}
	name="troubleshoot-flow"
	tainted={false}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: '🛠️ Troubleshoot a broken flow',
					description:
						'We created a flow that is a temperature converter that validates input and converts Celsius to Fahrenheit. For this tutorial, our flow is intentionally broken.',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: SELECTORS.testFlowButton,
				onHighlighted: async () => {
					stepComplete[1] = false
					await wait(DELAY_SHORT)
					stepComplete[1] = true
				},
				popover: {
					title: 'Test our flow',
					description:
						'Let\'s run it so you can see what needs to be fixed.',
					side: 'bottom',
					onNextClick: async () => {
						if (!checkStepComplete(1)) return

						// Click the Test Flow button to open the drawer
						const testFlowButton = document.querySelector(SELECTORS.testFlowButton) as HTMLElement
						if (testFlowButton) {
							testFlowButton.click()
							await wait(DELAY_LONG)
						}

						driver.moveNext()
					}
				}
			},
			{
				element: SELECTORS.testFlowDrawer,
				onHighlighted: async () => {
					stepComplete[2] = false
					await wait(DELAY_SHORT)
					stepComplete[2] = true
				},
				popover: {
					title: 'Run the flow',
					description:
						'Click "Next" to execute the flow. We\'ll use the results to troubleshoot the error.',
					side: 'left',
					onNextClick: async () => {
						if (!checkStepComplete(2)) return

						// Click the Test button to execute the flow
						const testButton = document.querySelector(SELECTORS.testFlowDrawer) as HTMLElement
						if (testButton) {
							testButton.click()
						}

						await wait(DELAY_LONG)
						driver.moveNext()
					}
				}
			},
			{
				element: '.border.rounded-md.shadow.p-2',
				onHighlighted: async () => {
					stepComplete[3] = false
					await wait(DELAY_SHORT)
					stepComplete[3] = true
				},
				popover: {
					title: 'Review the error',
					description:
						'Our flow failed. Let\'s review the error and understand what happened.',
					side: 'left',
					onNextClick: () => {
						if (!checkStepComplete(3)) return
						driver.moveNext()
					}
				}
			},
			{
				element: '.border-b.flex.flex-row.whitespace-nowrap.scrollbar-hidden.mx-auto',
				onHighlighted: async () => {
					stepComplete[4] = false
					await wait(DELAY_SHORT)
					stepComplete[4] = true
				},
				popover: {
					title: 'Explore the tabs',
					description:
						'Use these tabs to navigate between different views: Result, Logs, and Graph. We\'ll focus on the Graph tab to review the error.',
					side: 'bottom',
					onNextClick: () => {
						if (!checkStepComplete(4)) return
						driver.moveNext()
					}
				}
			},
			{
				element: '.grid.grid-cols-3.border.h-full',
				onHighlighted: async () => {
					stepComplete[5] = false
					await wait(DELAY_SHORT)

					// Find the step 'b' button inside the drawer and click it with fake cursor
					const flowPreviewContent = getElementBySelector(SELECTORS.flowPreviewContent)
					if (flowPreviewContent) {
						const stepButton = findButtonByText(flowPreviewContent, TEXT.convertToFahrenheit)

						if (stepButton) {
							await animateFakeCursorClick(stepButton, 1.5, { usePointerEvents: true })
							await wait(DELAY_MEDIUM)
						}
					}

					stepComplete[5] = true
				},
				popover: {
					title: 'Inspect the flow graph',
					description:
						'B step failed during the run. Let\'s take a closer look at its behavior.',
					side: 'top',
					onNextClick: () => {
						if (!checkStepComplete(5)) return
						driver.moveNext()
					}
				}
			},
			{
				element: '.rounded-md.grow.bg-surface-tertiary.text-xs.flex.flex-col.max-h-screen.gap-2.overflow-hidden.border',
				onHighlighted: async () => {
					stepComplete[6] = false
					await wait(DELAY_SHORT)
					stepComplete[6] = true
				},
				popover: {
					title: 'Error spotted!',
					description:
						'We made a typo in the code. Let\'s fix it and run the flow again.',
					side: 'left',
					onNextClick: async () => {
						if (!checkStepComplete(6)) return

						// Click the close button inside the drawer
						const drawer = getElementBySelector(SELECTORS.flowPreviewContent)
						if (drawer) {
							const closeButton = findCloseButton(drawer)

							if (closeButton) {
								await animateFakeCursorClick(closeButton, 1.5)
							}
						}

						await wait(DELAY_LONG)
						driver.moveNext()
					}
				}
			},
			{
				element: SELECTORS.stepB,
				onHighlighted: async () => {
					stepComplete[7] = false
					await wait(DELAY_SHORT)

					// Click on div id="b" to open the editor
					const stepBDiv = getElementBySelector(SELECTORS.stepB)
					if (stepBDiv) {
						await animateFakeCursorClick(stepBDiv, 1.5)
						await wait(DELAY_LONG)
					}

					stepComplete[7] = true
				},
				popover: {
					title: 'Your turn now!',
					description:
						'Fix the issue in the code, and run the flow again to confirm everything works.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu or in the <strong>Help</strong> submenu.</p>',
					side: 'top',
					onNextClick: () => {
						if (!checkStepComplete(7)) return
						updateProgress(index)
						driver.destroy()
					}
				}
			}
		]

		return steps
	}}
/>

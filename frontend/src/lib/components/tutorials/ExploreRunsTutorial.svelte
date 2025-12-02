<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow } from '$lib/gen'
	import { wait, type StateStore } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { triggerPointerDown } from './utils'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	// Flags to track if steps are complete
	let step1Complete = $state(false)
	let step2Complete = $state(false)
	let step3Complete = $state(false)
	let step4Complete = $state(false)
	let step5Complete = $state(false)
	let step6Complete = $state(false)
	let step7Complete = $state(false)

	// Constants for delays
	const DELAY_SHORT = 100
	const DELAY_MEDIUM = 300
	const DELAY_LONG = 500
	const DELAY_ANIMATION = 1500

	// Helper function to create and animate a fake cursor
	async function createFakeCursor(
		startElement: HTMLElement | null,
		endElement: HTMLElement,
		transitionDuration: number = 1.5
	): Promise<HTMLElement> {
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
			transition: all ${transitionDuration}s ease-in-out;
		`
		document.body.appendChild(fakeCursor)

		const endRect = endElement.getBoundingClientRect()
		let startX: number, startY: number

		if (startElement) {
			const startRect = startElement.getBoundingClientRect()
			startX = startRect.left + startRect.width / 2
			startY = startRect.top + startRect.height / 2
		} else {
			startX = endRect.left - 100
			startY = endRect.top + endRect.height / 2
		}

		fakeCursor.style.left = `${startX}px`
		fakeCursor.style.top = `${startY}px`

		await wait(100)

		fakeCursor.style.left = `${endRect.left + endRect.width / 2}px`
		fakeCursor.style.top = `${endRect.top + endRect.height / 2}px`

		await wait(transitionDuration * 1000)

		return fakeCursor
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
	index={10}
	name="explore-runs"
	tainted={false}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				element: '#flow-editor-test-flow',
				onHighlighted: async () => {
					step1Complete = false
					await wait(DELAY_SHORT)
					step1Complete = true
				},
				popover: {
					title: 'Troubleshoot a broken flow',
					description:
						'This flow is intentionally broken. Let’s run it with an input of 25°C so you can see what needs to be fixed.',
					side: 'bottom',
					onNextClick: async () => {
						if (!step1Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}

						// Click the Test Flow button to open the drawer
						const testFlowButton = document.querySelector('#flow-editor-test-flow') as HTMLElement
						if (testFlowButton) {
							testFlowButton.click()
							await wait(DELAY_LONG)
						}

						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-test-flow-drawer',
				onHighlighted: async () => {
					step2Complete = false
					await wait(DELAY_SHORT)
					step2Complete = true
				},
				popover: {
					title: 'Run the flow',
					description:
						'Click "Next" to execute the flow. We\'ll use the results to troubleshoot the error.',
					side: 'left',
					onNextClick: async () => {
						if (!step2Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}

						// Click the Test button to execute the flow
						const testButton = document.querySelector('#flow-editor-test-flow-drawer') as HTMLElement
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
					step3Complete = false
					await wait(DELAY_SHORT)
					step3Complete = true
				},
				popover: {
					title: 'Review the error',
					description:
						'Step b failed during the run. Let’s review the error and understand why this step didn’t work.',
					side: 'left',
					onNextClick: () => {
						if (!step3Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '.border-b.flex.flex-row.whitespace-nowrap.scrollbar-hidden.mx-auto',
				onHighlighted: async () => {
					step4Complete = false
					await wait(DELAY_SHORT)
					step4Complete = true
				},
				popover: {
					title: 'Explore the tabs',
					description:
						'Use these tabs to navigate between different views: Result, Logs, and Graph. We’ll focus on the Result tab to review the error.',
					side: 'bottom',
					onNextClick: () => {
						if (!step4Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '.grid.grid-cols-3.border.h-full',
				onHighlighted: async () => {
					step5Complete = false
					await wait(DELAY_SHORT)

					// Find the step 'b' button inside the drawer and click it with fake cursor
					const flowPreviewContent = document.getElementById('flow-preview-content')
					if (flowPreviewContent) {
						// Find the button containing "Validate temperature input" text
						const buttons = Array.from(flowPreviewContent.querySelectorAll('button'))
						const stepButton = buttons.find(btn =>
							btn.textContent?.includes('Convert to Fahrenheit')
						) as HTMLElement

						if (stepButton) {
							// Create fake cursor and animate it to the button
							const fakeCursor = await createFakeCursor(null, stepButton, 1.5)
							await wait(DELAY_MEDIUM)

							// Animate click (shrink cursor briefly)
							fakeCursor.style.transform = 'scale(0.8)'
							await wait(100)
							fakeCursor.style.transform = 'scale(1)'
							await wait(100)

							// Trigger pointer events (flow graph uses pointer events instead of click)
							stepButton.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
							stepButton.dispatchEvent(new PointerEvent('pointerup', { bubbles: true }))
							stepButton.click()
							await wait(DELAY_SHORT)

							// Remove fake cursor
							fakeCursor.remove()
							await wait(DELAY_MEDIUM)
						}
					}

					step5Complete = true
				},
				popover: {
					title: 'Inspect the flow graph',
					description:
						'Here’s the full execution graph. Let’s select step b—the one that failed—to take a closer look at its behavior.',
					side: 'top',
					onNextClick: () => {
						if (!step5Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '.rounded-md.grow.bg-surface-tertiary.text-xs.flex.flex-col.max-h-screen.gap-2.overflow-hidden.border',
				onHighlighted: async () => {
					step6Complete = false
					await wait(DELAY_SHORT)
					step6Complete = true
				},
				popover: {
					title: 'Check step details',
					description:
						'This panel shows the code, output, and logs for the selected step. It’s the best place to spot mistakes and understand how to fix them.',
					side: 'left',
					onNextClick: async () => {
						if (!step6Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}

						// Click the close button inside the drawer
						const drawer = document.getElementById('flow-preview-content')
						if (drawer) {
							const closeButton = Array.from(drawer.querySelectorAll('button')).find(btn => {
								const svg = btn.querySelector('svg.lucide-x')
								return svg !== null
							}) as HTMLElement

							if (closeButton) {
								// Create fake cursor and animate it to the close button
								const fakeCursor = await createFakeCursor(null, closeButton, 1.5)
								await wait(DELAY_MEDIUM)

								// Animate click (shrink cursor briefly)
								fakeCursor.style.transform = 'scale(0.8)'
								await wait(100)
								fakeCursor.style.transform = 'scale(1)'
								await wait(100)

								// Click the button
								closeButton.click()
								await wait(DELAY_SHORT)

								// Remove fake cursor
								fakeCursor.remove()
							}
						}

						await wait(DELAY_LONG)
						driver.moveNext()
					}
				}
			},
			{
				element: '#b',
				onHighlighted: async () => {
					step7Complete = false
					await wait(DELAY_SHORT)
					step7Complete = true
				},
				popover: {
					title: 'Your turn now!',
					description:
						'Open step b, fix the issue in the code, and run the flow again to confirm everything works.',
					side: 'top',
					onNextClick: () => {
						if (!step7Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.destroy()
					}
				}
			}
		]

		return steps
	}}
/>

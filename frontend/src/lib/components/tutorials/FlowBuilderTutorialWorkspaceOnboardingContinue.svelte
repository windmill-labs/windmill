<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { isFlowTainted, triggerPointerDown, clickButtonBySelector } from './utils'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import { loadFlowModuleState } from '../flows/flowStateUtils.svelte'
	import { wait } from '$lib/utils'
	import { get } from 'svelte/store'
	const { flowStore, flowStateStore, selectionManager, currentEditor } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial() {
		// Clear any existing flow drafts from localStorage to ensure fresh start
		try {
			localStorage.removeItem('flow')
		} catch (e) {
			console.error('Error clearing localStorage', e)
		}
		tutorial?.runTutorial()
	}

	const flowJson: Flow = {
		summary: '',
		description: '',
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
						}
					},
					summary: 'Validate temperature input'
				},
				{
					id: 'b',
					value: {
						type: 'rawscript',
						content:
							'export async function main(celsius: number) {\n  // Convert Celsius to Fahrenheit using the formula: F = (C × 9/5) + 32\n  const fahrenheit = (celsius * 9/5) + 32;\n  \n  return {\n    celsius: celsius,\n    fahrenheit: Math.round(fahrenheit * 100) / 100 // Round to 2 decimal places\n  };\n}',
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
					default: ""
				},
				city: {
					type: 'string',
					description: 'City you are now in',
					default: ""
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
	index={8}
	name="workspace-onboarding-continue"
	tainted={isFlowTainted(flowStore.val)}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to this flow builder tutorial',
					description:
						"In this tutorial, we will create a simple flow that validates a temperature in Celsius and converts it to Fahrenheit.",
					onNextClick: async () => {
						// Initialize empty flow with just the schema
						const emptyFlow: Flow = {
							summary: '',
							description: '',
							value: { modules: [] },
							schema: flowJson.schema,
							path: '',
							edited_at: '',
							edited_by: '',
							archived: false,
							extra_perms: {}
						}
						await initFlow(emptyFlow, flowStore as any, flowStateStore)

						// Add modules one by one with animation delays
						const modules = flowJson.value.modules
						for (let i = 0; i < modules.length; i++) {
							await new Promise((resolve) => setTimeout(resolve, i === 0 ? 0 : 700))
							
							const moduleData = modules[i]
							const module: FlowModule = {
								id: moduleData.id,
								summary: moduleData.summary,
								value: moduleData.value
							}
							
							// Load module state
							const state = await loadFlowModuleState(module)
							flowStateStore.val[module.id] = state
							
							// Add module to flow
							flowStore.val.value.modules.push(module)
							flowStore.val = { ...flowStore.val } // Trigger reactivity
						}

						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-virtual-Input',
				onHighlighted: async () => {
					// Click on the input button to open the drawer
					await wait(300)
					triggerPointerDown('#flow-editor-virtual-Input')
					await wait(100)
					selectionManager.selectId('Input')
					await wait(200)

					// Modify the driver.js overlay to only cover the left half
					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					// Simulate typing "25" slowly in celsius input
					const celsiusInput = document.querySelector('input[type="number"][placeholder=""]') as HTMLInputElement
					if (celsiusInput) {
						// Clear existing value first
						celsiusInput.value = ''
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(300)

						// Type "2"
						celsiusInput.value = '2'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(400)

						// Type "5"
						celsiusInput.value = '25'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
					}

					// Wait a bit then type "New York" in the city textarea
					await wait(500)
					const cityTextarea = document.querySelector('textarea[placeholder=""]') as HTMLTextAreaElement
					if (cityTextarea) {
						const text = 'New York'
						cityTextarea.value = ''
						cityTextarea.dispatchEvent(new Event('input', { bubbles: true }))
						cityTextarea.dispatchEvent(new Event('change', { bubbles: true }))
						await wait(200)

						// Type each character
						for (let i = 0; i < text.length; i++) {
							cityTextarea.value = text.substring(0, i + 1)
							cityTextarea.dispatchEvent(new Event('input', { bubbles: true }))
							cityTextarea.dispatchEvent(new Event('change', { bubbles: true }))
							await wait(5)
						}
					}
				},
				popover: {
					title: 'Flow inputs',
					description: 'Here, you give the input of your flow. It can be a strings, numbers, booleans, objects,.. Any data type that want your flow to use.',
					side: 'bottom',
					align: 'start',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#a',
				onHighlighted: async () => {
					// Click on the 'a' node to open the drawer
					selectionManager.selectId('a')
					await wait(500) // Wait for drawer to open and editor to load

					// Modify the driver.js overlay to only cover the left half
					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					// Wait for the editor to be available
					let editorAvailable = false
					let attempts = 0
					while (!editorAvailable && attempts < 20) {
						await wait(100)
						const editorState = get(currentEditor)
						if (editorState && editorState.type === 'script' && editorState.stepId === 'a') {
							editorAvailable = true
						}
						attempts++
					}

					const editorState = get(currentEditor)
					if (editorAvailable && editorState && editorState.type === 'script') {
						const editor = editorState.editor
						const moduleA = flowJson.value.modules.find(m => m.id === 'a')
						const codeToType = (moduleA?.value && 'content' in moduleA.value) ? moduleA.value.content : ''

						if (codeToType) {
							// Clear the editor first
							editor.setCode('', true)
							await wait(200)

							// Get Monaco editor model to access the underlying editor
							const model = editor.getModel()
							if (!model) {
								return
							}

							// Access the Monaco editor instance through the model
							// We'll use executeEdits for more reliable character insertion
							let currentText = ''

							// Type the code character by character
							for (let i = 0; i < codeToType.length; i++) {
								const char = codeToType[i]
								currentText += char

								// Use setCode with noHistory to update the editor
								editor.setCode(currentText, true)

								// Small delay between characters (slightly longer to ensure editor processes)
								const delay = char === '\n' ? 5 : 2
								await wait(delay)
							}
						}
					}
				},
				popover: {
					title: 'Let\'s write the code for our first script',
					description:
						"Our first check if the input temperature is valid.",
					side: 'bottom',
					onNextClick: () => {
						// Hide the default driver.js overlay
						const driverOverlay = document.querySelector('.driver-overlay') as HTMLElement
						if (driverOverlay) {
							driverOverlay.style.display = 'none'
						}

						// Create a single custom overlay with clip-path to reveal only bottom-right corner
						const customOverlay = document.createElement('div')
						customOverlay.className = 'tutorial-custom-overlay'
						customOverlay.style.cssText = `
							position: fixed;
							top: 0;
							left: 0;
							width: 100%;
							height: 100%;
							background-color: rgba(0, 0, 0, 0.5);
							z-index: 9999;
							pointer-events: none;
							clip-path: polygon(
								0 0, 100% 0, 100% 50%, 50% 50%, 50% 100%, 0 100%
							);
						`
						document.body.appendChild(customOverlay)

						driver.moveNext()
					}
				}
			},
			{
				onHighlighted: async () => {
					// Make the button visible by removing the opacity-0 class from its parent
					document.querySelector('#flow-editor-plug')?.parentElement?.classList.remove('opacity-0')
					// Click the button to open the connections panel
					await wait(100)
					clickButtonBySelector('#flow-editor-plug')

					// Wait for the connections panel to open
					await wait(800)

					// Find the target button with title="flow_input.celsius"
					const targetButton = document.querySelector('button[title="flow_input.celsius"]') as HTMLElement
					if (targetButton) {
						// Create a fake cursor element
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
							transition: all 2.5s ease-in-out;
						`
						document.body.appendChild(fakeCursor)

						// Get the plug button position (starting point)
						const plugButton = document.querySelector('#flow-editor-plug') as HTMLElement
						const startRect = plugButton?.getBoundingClientRect()

						// Get the target button position (ending point)
						const targetRect = targetButton.getBoundingClientRect()

						if (startRect && targetRect) {
							// Start cursor at plug button
							fakeCursor.style.left = `${startRect.left + startRect.width / 2}px`
							fakeCursor.style.top = `${startRect.top + startRect.height / 2}px`

							// Wait a frame for the initial position to be set
							await wait(100)

							// Move cursor to target
							fakeCursor.style.left = `${targetRect.left + targetRect.width / 2}px`
							fakeCursor.style.top = `${targetRect.top + targetRect.height / 2}px`

							// Wait for animation to complete
							await wait(2500)

							// Simulate click on the target button using dispatchEvent to avoid focus issues
							const clickEvent = new MouseEvent('click', {
								bubbles: true,
								cancelable: true,
								view: window
							})
							targetButton.dispatchEvent(clickEvent)

							// Remove fake cursor after clicking
							await wait(500)
							fakeCursor.remove()
						}
					}

					// Clean up custom overlay when moving to next step
					const cleanupOverlays = () => {
						document.querySelector('.tutorial-custom-overlay')?.remove()
						// Restore the driver.js overlay
						const driverOverlay = document.querySelector('.driver-overlay') as HTMLElement
						if (driverOverlay) {
							driverOverlay.style.display = ''
						}
					}

					// Store cleanup function for next step
					;(window as any).__tutorialCleanupOverlays = cleanupOverlays
				},
				popover: {
					title: 'Let\'s connect our script to the user input',
					description: 'We use data connector to get the user input, and pass it to our script.',
					onNextClick: () => {
						// Clean up overlays before moving to next step
						if ((window as any).__tutorialCleanupOverlays) {
							;(window as any).__tutorialCleanupOverlays()
							delete (window as any).__tutorialCleanupOverlays
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '#b',
				onHighlighted: async () => {
					// Click on the 'b' node to open the drawer
					selectionManager.selectId('b')
					await wait(500)

					// Make the plug button visible and click it to open the connections panel
					document.querySelector('#flow-editor-plug')?.parentElement?.classList.remove('opacity-0')
					await wait(100)
					clickButtonBySelector('#flow-editor-plug')

					// Wait for the connections panel to open
					await wait(800)

					// Find the target button with title="results.a"
					const targetButton = document.querySelector('button[title="results.a"]') as HTMLElement
					if (targetButton) {
						// Create a fake cursor element
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
							transition: all 2.5s ease-in-out;
						`
						document.body.appendChild(fakeCursor)

						// Get the plug button position (starting point)
						const plugButton = document.querySelector('#flow-editor-plug') as HTMLElement
						const startRect = plugButton?.getBoundingClientRect()

						// Get the target button position (ending point)
						const targetRect = targetButton.getBoundingClientRect()

						if (startRect && targetRect) {
							// Start cursor at plug button
							fakeCursor.style.left = `${startRect.left + startRect.width / 2}px`
							fakeCursor.style.top = `${startRect.top + startRect.height / 2}px`

							// Wait a frame for the initial position to be set
							await wait(100)

							// Move cursor to target
							fakeCursor.style.left = `${targetRect.left + targetRect.width / 2}px`
							fakeCursor.style.top = `${targetRect.top + targetRect.height / 2}px`

							// Wait for animation to complete (but don't click)
							await wait(2500)

							// Remove fake cursor without clicking
							await wait(500)
							fakeCursor.remove()
						}
					}
				},
				popover: {
					title: 'Convert to Fahrenheit',
					description: 'This script converts the celsius temperature into fahrenheit.',
					side: 'bottom',
					onNextClick: () => {
						// Clean up overlays before moving to next step
						if ((window as any).__tutorialCleanupOverlays) {
							;(window as any).__tutorialCleanupOverlays()
							delete (window as any).__tutorialCleanupOverlays
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-test-flow',
				onHighlighted: async () => {
					// Restore the overlay to full width
					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.width = ''
						overlay.style.right = ''
						overlay.style.left = ''
					}
				},
				popover: {
					title: 'Test your flow',
					description: 'This is the test button. It will execute your flow and show the results.',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				onHighlighted: async () => {
					// Create countdown element
					const popoverDescription = document.querySelector('#driver-popover-description')
					let countdownElement: HTMLElement | null = null
					if (popoverDescription) {
						countdownElement = document.createElement('div')
						countdownElement.style.cssText = 'margin-top: 8px; font-size: 12px; color: #6b7280; font-style: italic;'
						popoverDescription.appendChild(countdownElement)
					}

					let secondsLeft = 5
					const updateCountdown = () => {
						if (countdownElement) {
							countdownElement.textContent = `Finishing in ${secondsLeft} second${secondsLeft !== 1 ? 's' : ''}...`
						}
					}

					updateCountdown()
					const countdownInterval = setInterval(() => {
						secondsLeft--
						if (secondsLeft > 0) {
							updateCountdown()
						} else {
							clearInterval(countdownInterval)
							if (countdownElement) {
								countdownElement.remove()
							}
							driver.destroy()
						}
					}, 1000)

					// Store interval reference to clear it if user clicks Next
					;(window as any).__tutorialAutoFinishInterval = countdownInterval
				},
				popover: {
					title: 'Your turn now',
					description: 'Insert a temperature in Celsius and click test your flow to see the results.',
					onNextClick: () => {
						// Clear the countdown interval if it exists
						const interval = (window as any).__tutorialAutoFinishInterval
						if (interval) {
							clearInterval(interval)
							delete (window as any).__tutorialAutoFinishInterval
						}
						driver.destroy()
					}
				}
			}
		]

		return steps
	}}
/>

<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { isFlowTainted, triggerPointerDown, clickButtonBySelector } from './utils'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import { loadFlowModuleState } from '../flows/flowStateUtils.svelte'
	import { wait, type StateStore } from '$lib/utils'
	import { get } from 'svelte/store'
	const { flowStore, flowStateStore, selectionManager, currentEditor } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

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

	export function runTutorial() {
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
						input_transforms: {}
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
					title: 'Welcome to this flow tutorial',
					description:
						"In this tutorial, we will create a simple flow that validates a temperature in Celsius and converts it to Fahrenheit.",
					onNextClick: async () => {
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
						await initFlow(emptyFlow, flowStore as StateStore<Flow>, flowStateStore)

						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-virtual-Input',
				onHighlighted: async () => {
					await wait(300)
					triggerPointerDown('#flow-editor-virtual-Input')
					await wait(100)
					selectionManager.selectId('Input')
					await wait(200)

					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					const celsiusInput = document.querySelector('input[type="number"][placeholder=""]') as HTMLInputElement
					if (celsiusInput) {
						celsiusInput.value = ''
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(300)

						celsiusInput.value = '2'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(400)

						celsiusInput.value = '25'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
					}
				},
				popover: {
					title: 'Define the User Input',
					description: 'First, define the data the flow will receive. For this example, we\'ll configure the input to accept a temperature in Celsius.',
					side: 'bottom',
					align: 'start',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-add-step-0',
				onHighlighted: async () => {
					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.display = 'none'
					}

					await wait(500)
					const button = document.querySelector('#flow-editor-add-step-0') as HTMLElement
					if (button) {
						button.click()
					}

					await wait(800)

					const spans = Array.from(document.querySelectorAll('span'))
					const bunSpan = spans.find(span => span.textContent?.includes('TypeScript (Bun)')) as HTMLElement

					if (bunSpan) {
						const fakeCursor = await createFakeCursor(null, bunSpan, 1.5)
						await wait(1000)
						fakeCursor.remove()

						// Automatically trigger next step after cursor animation
						await wait(500)

						// Restore overlay
						const overlay = document.querySelector('.driver-overlay') as HTMLElement
						if (overlay) {
							overlay.style.display = ''
						}

						// Add module
						const moduleData = flowJson.value.modules[0]
						const module: FlowModule = {
							id: moduleData.id,
							summary: moduleData.summary,
							value: moduleData.value
						}

						const state = await loadFlowModuleState(module)
						flowStateStore.val[module.id] = state

						flowStore.val.value.modules.push(module)
						flowStore.val = { ...flowStore.val }

						await wait(700)

						driver.moveNext()
					}
				},
				popover: {
					title: 'Select a language',
					description: 'Now we need to create our first script. We\'ll use TypeScript for this example.',
					side: 'top'
				}
			},
			{
				element: '#a',
				onHighlighted: async () => {
					selectionManager.selectId('a')
					await wait(500)

					const overlay = document.querySelector('.driver-overlay') as HTMLElement
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					let editorState = get(currentEditor)
					let attempts = 0
					while (attempts < 20) {
						if (editorState && editorState.type === 'script' && editorState.stepId === 'a') {
							break
						}
						await wait(100)
						editorState = get(currentEditor)
						attempts++
					}

					if (editorState && editorState.type === 'script') {
						const editor = editorState.editor
						const moduleA = flowJson.value.modules.find(m => m.id === 'a')
						const codeToType = (moduleA?.value && 'content' in moduleA.value) ? moduleA.value.content : ''

						if (codeToType) {
							editor.setCode('', true)
							await wait(200)

							let currentText = ''
							for (let i = 0; i < codeToType.length; i++) {
								const char = codeToType[i]
								currentText += char
								editor.setCode(currentText, true)
								const delay = char === '\n' ? 5 : 2
								await wait(delay)
							}
						}
					}
				},
				popover: {
					title: 'Write our script',
					description:
						"Then, we write the code for this script. Its purpose is to collect the temperature input and determine if it is a valid value.",
					side: 'bottom',
					onNextClick: () => {
						const driverOverlay = document.querySelector('.driver-overlay') as HTMLElement
						if (driverOverlay) {
							driverOverlay.style.display = 'none'
						}

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
					document.querySelector('#flow-editor-plug')?.parentElement?.classList.remove('opacity-0')
					await wait(100)
					clickButtonBySelector('#flow-editor-plug')

					await wait(800)

					const targetButton = document.querySelector('button[title="flow_input.celsius"]') as HTMLElement
					if (targetButton) {
						const plugButton = document.querySelector('#flow-editor-plug') as HTMLElement
						if (plugButton) {
							const fakeCursor = await createFakeCursor(plugButton, targetButton, 2.5)
							const clickEvent = new MouseEvent('click', {
								bubbles: true,
								cancelable: true,
								view: window
							})
							targetButton.dispatchEvent(clickEvent)
							await wait(500)
							fakeCursor.remove()
						}
					}
				},
				popover: {
					title: 'Connect input data to the script',
					description: 'Now we need to connect the input data to our script. We use data connectors to pass data between our flow steps.',
					onNextClick: async () => {
						driver.moveNext()
					}
				}
			},
			{
				onHighlighted: async () => {
					await wait(500)
					const buttons = Array.from(document.querySelectorAll('button'))
					const testTabButton = buttons.find(btn => {
						return btn.textContent?.includes('Test this step') &&
							btn.classList.contains('border-b-2') &&
							btn.classList.contains('cursor-pointer')
					}) as HTMLElement

					if (testTabButton) {
						// Animate cursor to Test this step tab
						const fakeCursor1 = await createFakeCursor(null, testTabButton, 1.5)
						await wait(300)
						testTabButton.click()
						fakeCursor1.remove()
					}

					await wait(800)

					const testActionButton = Array.from(document.querySelectorAll('button')).find(btn => {
						return btn.textContent?.includes('Run') &&
							btn.classList.contains('bg-surface-accent-primary') &&
							btn.classList.contains('w-full')
					}) as HTMLElement

					if (testActionButton) {
						// Animate cursor from Test this step to Run button
						const testTabButton2 = buttons.find(btn => {
							return btn.textContent?.includes('Test this step') &&
								btn.classList.contains('border-b-2') &&
								btn.classList.contains('cursor-pointer')
						}) as HTMLElement

						const fakeCursor2 = await createFakeCursor(testTabButton2, testActionButton, 1.5)
						await wait(300)
						testActionButton.click()
						fakeCursor2.remove()
					}
				},
				popover: {
					title: 'Test the script',
					description: 'We test the script to ensure the validation logic is working correctly. Once validated, we to complete our flow with scripts b and c.',
					onNextClick: async () => {
						// Clean up custom overlay immediately
						const customOverlay = document.querySelector('.tutorial-custom-overlay')
						if (customOverlay) {
							customOverlay.remove()
						}

						// Reset the driver.js overlay to full screen
						const driverOverlay = document.querySelector('.driver-overlay') as HTMLElement
						if (driverOverlay) {
							driverOverlay.style.display = ''
							driverOverlay.style.width = ''
							driverOverlay.style.right = ''
							driverOverlay.style.left = ''
						}

						const modulesToAdd = [flowJson.value.modules[1], flowJson.value.modules[2]]
						for (let i = 0; i < modulesToAdd.length; i++) {
							await new Promise((resolve) => setTimeout(resolve, i === 0 ? 0 : 700))

							const moduleData = modulesToAdd[i]
							const module: FlowModule = {
								id: moduleData.id,
								summary: moduleData.summary,
								value: moduleData.value
							}

							const state = await loadFlowModuleState(module)
							flowStateStore.val[module.id] = state

							flowStore.val.value.modules.push(module)
							flowStore.val = { ...flowStore.val }
						}

						await wait(700)

						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-test-flow',
				popover: {
					title: 'The flow is now complete!',
					description: 'Click the Test Flow button to execute the entire process and view the final results.',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
		]

		return steps
	}}
/>

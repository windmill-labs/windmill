<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import {
		isFlowTainted,
		triggerPointerDown,
		clickButtonBySelector,
		DELAY_SHORT,
		DELAY_MEDIUM,
		DELAY_LONG,
		DELAY_ANIMATION,
		DELAY_ANIMATION_LONG,
		DELAY_TYPING,
		DELAY_CODE_CHAR,
		DELAY_CODE_NEWLINE,
		moveCursorToElement,
		createFakeCursor
	} from './utils'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import { loadFlowModuleState } from '../flows/flowStateUtils.svelte'
	import { wait, type StateStore } from '$lib/utils'
	import { get } from 'svelte/store'
	import { sendUserToast } from '$lib/toast'
	import { updateProgress } from '$lib/tutorialUtils'
	const { flowStore, flowStateStore, selectionManager, currentEditor } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = undefined

	// Flags to track if steps are complete
	let step2Complete = $state(false)
	let step3Complete = $state(false)
	let step4Complete = $state(false)
	let step5Complete = $state(false)
	let step6Complete = $state(false)

	// Helper function to get driver overlay
	function getDriverOverlay(): HTMLElement | null {
		return document.querySelector('.driver-overlay') as HTMLElement | null
	}

	// Helper function to type text character by character
	async function typeText(input: HTMLInputElement, text: string, delay: number = DELAY_TYPING): Promise<void> {
		input.value = ''
		input.focus()
		for (let i = 0; i < text.length; i++) {
			input.value += text[i]
			input.dispatchEvent(new Event('input', { bubbles: true }))
			await wait(delay)
		}
	}

	// Helper function to update module summary in flowStore
	function updateModuleSummary(moduleId: string, summary: string): void {
		const moduleIndex = flowStore.val.value.modules.findIndex(m => m.id === moduleId)
		if (moduleIndex !== -1) {
			flowStore.val.value.modules[moduleIndex].summary = summary
			flowStore.val = { ...flowStore.val }
		}
	}

	// Helper function to add module to flow
	async function addModuleToFlow(module: FlowModule): Promise<void> {
		const state = await loadFlowModuleState(module)
		flowStateStore.val[module.id] = state
		flowStore.val.value.modules.push(module)
		flowStore.val = { ...flowStore.val }
	}

	// Helper function to find button by text and classes
	function findButtonByText(text: string, classes: string[] = []): HTMLElement | null {
		const buttons = Array.from(document.querySelectorAll('button'))
		return buttons.find(btn => {
			const hasText = btn.textContent?.includes(text) ?? false
			const hasClasses = classes.every(cls => btn.classList.contains(cls))
			return hasText && (classes.length === 0 || hasClasses)
		}) as HTMLElement | null
	}

	// Helper function to cleanup custom overlay
	function cleanupCustomOverlay(): void {
		const customOverlay = document.querySelector('.tutorial-custom-overlay')
		if (customOverlay) {
			customOverlay.remove()
		}
	}

	// Helper function to create and animate a fake cursor (extended version with start element support)
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
	index={index}
	name="flow-live-tutorial"
	tainted={isFlowTainted(flowStore.val)}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Build your first flow',
					description:
						"Let's create a temperature converter that validates input and converts Celsius to Fahrenheit.",
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
					step2Complete = false

					await wait(DELAY_MEDIUM)
					triggerPointerDown('#flow-editor-virtual-Input')
					await wait(DELAY_SHORT)
					selectionManager.selectId('Input')
					await wait(200)

					const overlay = getDriverOverlay()
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					const celsiusInput = document.querySelector('input[type="number"][placeholder=""]') as HTMLInputElement
					if (celsiusInput) {
						celsiusInput.value = ''
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(DELAY_MEDIUM)

						celsiusInput.value = '2'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))
						await wait(400)

						celsiusInput.value = '25'
						celsiusInput.dispatchEvent(new Event('input', { bubbles: true }))

						step2Complete = true
					}
				},
				popover: {
					title: 'Set the input',
					description: 'Every flow starts with input. Here we define a temperature in Celsius.',
					side: 'bottom',
					align: 'start',
					onNextClick: () => {
						if (!step2Complete) {
							sendUserToast('Please wait for the input to be filled...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-add-step-0',
				onHighlighted: async () => {
					step3Complete = false

					// Animate cursor to the add step button
					const button = document.querySelector('#flow-editor-add-step-0') as HTMLElement
					if (button) {
						const fakeCursor1 = await createFakeCursorWithStart(null, button, 1.5)
						await wait(DELAY_SHORT)
						button.click()
						fakeCursor1.remove()
					}

					const overlay = getDriverOverlay()
					if (overlay) {
						overlay.style.display = 'none'
					}

					await wait(DELAY_LONG)

					const spans = Array.from(document.querySelectorAll('span'))
					const bunSpan = spans.find(span => span.textContent?.includes('TypeScript (Bun)')) as HTMLElement

					if (bunSpan) {
						// Animate cursor from add step button to TypeScript (Bun) span
						const fakeCursor2 = await createFakeCursorWithStart(button, bunSpan, 1.5)
						await wait(DELAY_MEDIUM)
						fakeCursor2.remove()

						// Automatically trigger next step after cursor animation
						await wait(DELAY_SHORT)

						// Add module with empty summary and empty content
						const moduleData = flowJson.value.modules[0]
						const module: FlowModule = {
							id: moduleData.id,
							summary: '', // Start with empty summary
							value: moduleData.value
						}
						// Clear content after module creation if it's a rawscript
						if ('content' in module.value) {
							module.value = { ...module.value, content: '' } as typeof module.value
						}

						await addModuleToFlow(module)

						await wait(700)

						// Restore overlay
						const overlay = getDriverOverlay()
						if (overlay) {
							overlay.style.display = ''
						}

						step3Complete = true
						driver.moveNext()
					}
				},
				popover: {
					title: 'Choose TypeScript',
					description: 'Pick TypeScript (Bun) to write our validation script.',
					side: 'top',
					onNextClick: () => {
						if (!step3Complete) {
							sendUserToast('Please wait for the script to be created...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					},
					onPrevClick: () => {
						sendUserToast('Previous is not available for this step', true, [], undefined, 3000)
					}
				}
			},
			{
				element: '#a',
				onHighlighted: async () => {
					// Reset the flag when step starts
					step4Complete = false

					selectionManager.selectId('a')
					await wait(DELAY_LONG)

					const overlay = getDriverOverlay()
					if (overlay) {
						overlay.style.width = '50%'
						overlay.style.right = 'auto'
						overlay.style.left = '0'
					}

					// First, type the summary
					await wait(DELAY_MEDIUM)
					const summaryInput = document.querySelector('input[placeholder="Summary"]') as HTMLInputElement
					if (summaryInput) {
						const summaryText = 'Validate temperature input'
						await typeText(summaryInput, summaryText)
						updateModuleSummary('a', summaryText)
						await wait(DELAY_LONG)
					}

					// Then, type the code
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
								const delay = char === '\n' ? DELAY_CODE_NEWLINE : DELAY_CODE_CHAR
								await wait(delay)
							}

							// Update the flow store with the typed code
							const moduleIndex = flowStore.val.value.modules.findIndex(m => m.id === 'a')
							if (moduleIndex !== -1 && 'content' in flowStore.val.value.modules[moduleIndex].value) {
								flowStore.val.value.modules[moduleIndex].value = {
									...flowStore.val.value.modules[moduleIndex].value,
									content: codeToType
								}
								flowStore.val = { ...flowStore.val }
							}

							// Press Enter after finishing typing
							await wait(DELAY_MEDIUM)
							const model = editor.getModel()
							if (model && 'setValue' in model) {
								model.setValue(currentText + '\n')
							}

							// Mark step 4 as complete
							step4Complete = true
						}
					}
				},
				popover: {
					title: 'Add validation logic',
					description:
						"Watch as we write code to validate the temperature input.",
					side: 'bottom',
					onNextClick: () => {
						// Only proceed if code writing is complete
						if (!step4Complete) {
							sendUserToast('Please wait for the code to finish typing...', false, [], undefined, 3000)
							return
						}

						const driverOverlay = getDriverOverlay()
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
					},
					onPrevClick: () => {
						sendUserToast('Previous is not available for this step', true, [], undefined, 3000)
					}
				}
			},
			{
				onHighlighted: async () => {
					step5Complete = false

					// Create a single cursor that will move continuously
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

					// Step 1: Move to and click plug button
					document.querySelector('#flow-editor-plug')?.parentElement?.classList.remove('opacity-0')
					await wait(DELAY_SHORT)
					const plugButton = document.querySelector('#flow-editor-plug') as HTMLElement
					if (plugButton) {
						const plugRect = plugButton.getBoundingClientRect()
						// Start from off-screen left
						fakeCursor.style.left = `${plugRect.left - 100}px`
						fakeCursor.style.top = `${plugRect.top + plugRect.height / 2}px`
						await wait(DELAY_SHORT)
						// Move to plug button
						fakeCursor.style.left = `${plugRect.left + plugRect.width / 2}px`
						fakeCursor.style.top = `${plugRect.top + plugRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						clickButtonBySelector('#flow-editor-plug')
					}

					await wait(DELAY_MEDIUM)

					// Step 2: Move to and click flow_input.celsius
					const targetButton = document.querySelector('button[title="flow_input.celsius"]') as HTMLElement
					if (targetButton) {
						await moveCursorToElement(fakeCursor, targetButton, DELAY_ANIMATION_LONG)
						await wait(DELAY_MEDIUM)
						const clickEvent = new MouseEvent('click', {
							bubbles: true,
							cancelable: true,
							view: window
						})
						targetButton.dispatchEvent(clickEvent)
					}

					await wait(DELAY_LONG)

					// Step 3: Move to and click Test this step tab
					const testTabButton = findButtonByText('Test this step', ['border-b-2', 'cursor-pointer'])

					if (testTabButton) {
						await moveCursorToElement(fakeCursor, testTabButton, DELAY_ANIMATION)
						await wait(DELAY_SHORT)
						testTabButton.click()
					}

					await wait(DELAY_LONG)

					// Step 4: Move to and click Run button
					const testActionButton = findButtonByText('Run', ['bg-surface-accent-primary', 'w-full'])

					if (testActionButton) {
						await moveCursorToElement(fakeCursor, testActionButton, DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						testActionButton.click()
						await wait(DELAY_MEDIUM)
					}

					// Remove cursor at the end
					fakeCursor.remove()

					step5Complete = true
				},
				popover: {
					title: 'Wire it up and test',
					description: 'Connect the input, then run a quick test to verify the validation works.',
					onNextClick: async () => {
						if (!step5Complete) {
							sendUserToast('Please wait for the test to complete...', false, [], undefined, 3000)
							return
						}
						cleanupCustomOverlay()
						driver.moveNext()
					},
					onPrevClick: () => {
						sendUserToast('Previous is not available for this step', true, [], undefined, 3000)
					}
				}
			},
			{
				onHighlighted: async () => {
					step6Complete = false

					// First, add modules b and c with empty summaries
					const modulesToAdd = [flowJson.value.modules[1], flowJson.value.modules[2]]
					for (let i = 0; i < modulesToAdd.length; i++) {
						await new Promise((resolve) => setTimeout(resolve, i === 0 ? 0 : 700))

						const moduleData = modulesToAdd[i]
						const module: FlowModule = {
							id: moduleData.id,
							summary: '', // Start with empty summary
							value: moduleData.value
						}

						await addModuleToFlow(module)
					}

					await wait(700)

					// Create a single cursor for continuous movement
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

					// Step 1: Click on script 'b'
					await wait(DELAY_MEDIUM)
					const scriptB = document.querySelector('#b') as HTMLElement
					if (scriptB) {
						const bRect = scriptB.getBoundingClientRect()
						// Start from off-screen
						fakeCursor.style.left = `${bRect.left - 100}px`
						fakeCursor.style.top = `${bRect.top + bRect.height / 2}px`
						await wait(DELAY_SHORT)
						// Move to script b
						fakeCursor.style.left = `${bRect.left + bRect.width / 2}px`
						fakeCursor.style.top = `${bRect.top + bRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						selectionManager.selectId('b')
					}

					await wait(DELAY_LONG)

					// Type summary for script 'b'
					const summaryInputB = document.querySelector('input[placeholder="Summary"]') as HTMLInputElement
					if (summaryInputB) {
						const summaryTextB = 'Convert to Fahrenheit'
						await typeText(summaryInputB, summaryTextB)
						updateModuleSummary('b', summaryTextB)
						await wait(DELAY_LONG)
					}

					// Step 2: Move to and click on script 'c'
					const scriptC = document.querySelector('#c') as HTMLElement
					if (scriptC) {
						await moveCursorToElement(fakeCursor, scriptC, DELAY_ANIMATION)
						await wait(DELAY_SHORT)
						selectionManager.selectId('c')
					}

					await wait(DELAY_LONG)

					// Type summary for script 'c'
					const summaryInputC = document.querySelector('input[placeholder="Summary"]') as HTMLInputElement
					if (summaryInputC) {
						const summaryTextC = 'Categorize temperature'
						await typeText(summaryInputC, summaryTextC)
						updateModuleSummary('c', summaryTextC)
						await wait(DELAY_LONG)
					}

					// Move cursor to Test Flow button
					const testFlowButton = document.querySelector('#flow-editor-test-flow') as HTMLElement
					if (testFlowButton) {
						await moveCursorToElement(fakeCursor, testFlowButton, DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
					}

					// Remove cursor at the end
					fakeCursor.remove()

					step6Complete = true
				},
				popover: {
					title: 'Add the final steps',
					description: 'Two more scripts to convert and categorize the temperature.',
					onNextClick: () => {
						if (!step6Complete) {
							sendUserToast('Please wait for the summaries to be added...', false, [], undefined, 3000)
							return
						}

						// Reset the driver.js overlay to full screen
						const driverOverlay = getDriverOverlay()
						if (driverOverlay) {
							driverOverlay.style.display = ''
							driverOverlay.style.width = ''
							driverOverlay.style.right = ''
							driverOverlay.style.left = ''
						}
						driver.moveNext()
					},
					onPrevClick: () => {
						sendUserToast('Previous is not available for this step', true, [], undefined, 3000)
					}
				}
			},
			{
				element: '#flow-editor-test-flow',
				popover: {
					title: 'Ready to test!',
					description: 'Run the complete flow and see your temperature converter in action.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu or in the <strong>Help</strong> submenu.</p>',
					onNextClick: () => {
						updateProgress(index)
						driver.destroy()
					},
					onPrevClick: () => {
						sendUserToast('Previous is not available for this step', true, [], undefined, 3000)
					}
				}
			},
		]

		return steps
	}}
/>

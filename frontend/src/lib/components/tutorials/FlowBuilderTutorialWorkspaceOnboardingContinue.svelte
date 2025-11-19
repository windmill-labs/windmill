<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { isFlowTainted, triggerPointerDown } from './utils'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import { loadFlowModuleState } from '../flows/flowStateUtils.svelte'
	import { wait } from '$lib/utils'
	import { get } from 'svelte/store'
	const { flowStore, flowStateStore, selectedId, currentEditor } = getContext<FlowEditorContext>('FlowEditorContext')

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
	index={8}
	name="workspace-onboarding-continue"
	tainted={isFlowTainted(flowStore.val)}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Now let\'s create a flow together',
					description:
						"We will create together a simple flow that validates a temperature in Celsius and converts it to Fahrenheit.",
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
				element: '#a',
				popover: {
					title: 'This is a script',
					description:
						"To make our flow, we connected 3 scripts together : 'a', 'b' and 'c'. Each script executes a specific task.",
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#b',
				onHighlighted: async () => {
					// Click on the 'b' node to open the drawer
					selectedId.set('b')
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
						if (editorState && editorState.type === 'script' && editorState.stepId === 'b') {
							editorAvailable = true
						}
						attempts++
					}
					
					const editorState = get(currentEditor)
					if (editorAvailable && editorState && editorState.type === 'script') {
						const editor = editorState.editor
						const moduleB = flowJson.value.modules.find(m => m.id === 'b')
						const codeToType = (moduleB?.value && 'content' in moduleB.value) ? moduleB.value.content : ''
						
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
								const delay = char === '\n' ? 10 : 5
								await wait(delay)
							}
						}
					}
				},
				popover: {
					title: 'When you click on a script, it opens the code editor',
					description: 'On the top, you have the code of the script. On the bottom, you have data connectors with previous scripts. We use scripts ids to refer previous scripts data outputs.',
					onNextClick: () => {
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
					selectedId.set('Input')
					await wait(200)
				},
				popover: {
					title: 'Flow inputs',
					description: 'Here, you give the input of your flow. It can be a strings, numbers, booleans, objects,.. Any data type that want your flow to use.',
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
					description: 'Insert a temperature in Celsius and click on the Run button to execute your flow.',
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


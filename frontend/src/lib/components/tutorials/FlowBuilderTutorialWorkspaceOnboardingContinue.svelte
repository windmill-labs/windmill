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

	const { flowStore, flowStateStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	function hideOverlay() {
		const overlay = document.querySelector('.driver-overlay') as HTMLElement
		if (overlay) {
			overlay.style.display = 'none'
		}
	}

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
						lock: '{\n  "dependencies": {}\n}\n//bun.lock\n<empty>',
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
						lock: '{\n  "dependencies": {}\n}\n//bun.lock\n<empty>',
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
						lock: '{\n  "dependencies": {}\n}\n//bun.lock\n<empty>',
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
				onHighlighted: () => {
					hideOverlay()
				},
				popover: {
					title: 'Now let\'s create a flow together',
					description:
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Let\'s build your first flow step by step!</p>',
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
				onHighlighted: async () => {
					hideOverlay()

					// Automatically highlight each script in sequence with visual effects
					const highlightClass = 'tutorial-highlight-script'
					const style = document.createElement('style')
					style.textContent = `
						.${highlightClass} {
							outline: 3px solid #3b82f6 !important;
							outline-offset: 2px !important;
							border-radius: 4px !important;
							transition: outline 0.3s ease !important;
						}
					`
					document.head.appendChild(style)

					const elements = [
						{ selector: '#a', id: 'a' },
						{ selector: '#b', id: 'b' },
						{ selector: '#c', id: 'c' }
					]
					
					// Highlight each element in sequence and open its drawer
					for (let i = 0; i < elements.length; i++) {
						const { selector, id } = elements[i]
						const element = document.querySelector(selector) as HTMLElement
						if (element) {
							// Open the drawer for this script node
							selectedId.set(id)
							await wait(100) // Small delay to ensure drawer opens
							
							// Add visual highlight
							element.classList.add(highlightClass)
							await wait(800)
							element.classList.remove(highlightClass)
						}
					}
					
					// Clean up
					document.head.removeChild(style)
				},
				popover: {
					title: 'To make our flow, we connected 3 scripts together',
					description:
						'<p>We created three steps that work together:<br/>• Step <strong>a</strong>: Validates the temperature input<br/>• Step <strong>b</strong>: Converts Celsius to Fahrenheit<br/>• Step <strong>c</strong>: Categorizes the temperature</p>',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#flow-editor-virtual-Input',
				onHighlighted: () => {
					hideOverlay()
				},
				popover: {
					title: 'Flow inputs',
					description: 'Here you can define the inputs for your flow. These inputs can be used throughout your flow steps.',
					onNextClick: async () => {
						// Small delay before opening the drawer
						await wait(300)
						// Trigger the input button to open the drawer
						triggerPointerDown('#flow-editor-virtual-Input')
						await wait(100)
						// Set selectedId to 'Input' to ensure the drawer opens
						selectedId.set('Input')
						await wait(200)
						driver.moveNext()
					}
				}
			}
		]

		return steps
	}}
/>


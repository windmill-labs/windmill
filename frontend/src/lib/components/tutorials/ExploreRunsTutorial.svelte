<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { initFlow } from '../flows/flowStore.svelte'
	import type { Flow } from '$lib/gen'
	import { wait, type StateStore } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	// Flags to track if steps are complete
	let step1Complete = $state(false)
	let step2Complete = $state(false)
	let step3Complete = $state(false)

	// Constants for delays
	const DELAY_SHORT = 100
	const DELAY_MEDIUM = 300
	const DELAY_LONG = 500

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
					title: 'Test your flow',
					description:
						'Your temperature converter flow is ready with an input of 25°C. Let\'s test it!',
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
						'Click "Next" to execute the flow. You\'ll see it validate the temperature, convert to Fahrenheit, and categorize the result!',
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

						driver.destroy()
						sendUserToast(
							'Flow is running! Watch the execution and explore the results.',
							false,
							[],
							undefined,
							5000
						)
					}
				}
			}
		]

		return steps
	}}
/>

<script lang="ts">
	import { getContext } from 'svelte'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import type { Job } from '$lib/gen'
	import { wait } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		onLoad?: (job: Job) => void
	}

	let { onLoad }: Props = $props()

	let tutorial: Tutorial | undefined = undefined

	// Flags to track if steps are complete
	let step1Complete = $state(false)
	let step2Complete = $state(false)
	let step3Complete = $state(false)
	let step4Complete = $state(false)

	// Constants for delays
	const DELAY_SHORT = 100
	const DELAY_MEDIUM = 300
	const DELAY_LONG = 500

	// Mock job data - a completed flow run with the temperature converter
	const mockJob: Job = {
		type: 'CompletedJob',
		workspace_id: 'demo',
		id: '019ada00-0000-0000-0000-000000000001',
		parent_job: undefined,
		created_by: 'tutorial@windmill.dev',
		created_at: new Date(Date.now() - 5000).toISOString(),
		started_at: new Date(Date.now() - 4000).toISOString(),
		duration_ms: 1250,
		success: true,
		script_path: undefined,
		script_hash: undefined,
		args: {
			celsius: 25
		},
		result: {
			celsius: 25,
			fahrenheit: 77,
			category: 'Warm',
			emoji: '☀️'
		},
		logs: `job=019ada00-0000-0000-0000-000000000001 tag=flow worker=wk-default-tutorial

Starting flow execution...

[Step a] Validate temperature input
  Checking temperature: 25°C
  ✓ Temperature is valid (above absolute zero)
  ✓ Temperature is within reasonable range
  Result: { celsius: 25, isValid: true, message: "Temperature is valid" }
  Duration: 145ms

[Step b] Convert to Fahrenheit
  Converting 25°C to Fahrenheit
  Formula: F = (C × 9/5) + 32
  Result: { celsius: 25, fahrenheit: 77 }
  Duration: 89ms

[Step c] Categorize temperature
  Analyzing temperature: 25°C / 77°F
  Category: Warm
  Emoji: ☀️
  Result: { celsius: 25, fahrenheit: 77, category: "Warm", emoji: "☀️" }
  Duration: 112ms

Flow completed successfully in 1.25s
`,
		deleted: false,
		raw_code: undefined,
		canceled: false,
		canceled_by: undefined,
		canceled_reason: undefined,
		job_kind: 'flow',
		schedule_path: undefined,
		permissioned_as: 'u/tutorial',
		flow_status: {
			step: 2,
			modules: [
				{
					id: 'a',
					job: '019ada00-0000-0000-0000-000000000002',
					count: undefined,
					iterator: undefined,
					flow_jobs: undefined,
					branch_chosen: undefined,
					branchall: undefined,
					approvers: undefined,
					failed_retries: [],
					skipped: undefined
				},
				{
					id: 'b',
					job: '019ada00-0000-0000-0000-000000000003',
					count: undefined,
					iterator: undefined,
					flow_jobs: undefined,
					branch_chosen: undefined,
					branchall: undefined,
					approvers: undefined,
					failed_retries: [],
					skipped: undefined
				},
				{
					id: 'c',
					job: '019ada00-0000-0000-0000-000000000004',
					count: undefined,
					iterator: undefined,
					flow_jobs: undefined,
					branch_chosen: undefined,
					branchall: undefined,
					approvers: undefined,
					failed_retries: [],
					skipped: undefined
				}
			],
			failure_module: {
				id: undefined,
				job: undefined,
				count: undefined,
				iterator: undefined,
				flow_jobs: undefined,
				branch_chosen: undefined,
				branchall: undefined,
				approvers: undefined,
				failed_retries: undefined,
				skipped: undefined,
				parent_module: undefined
			},
			preprocessor_module: undefined,
			retry: {
				fail_count: 0,
				failed_jobs: []
			},
			cleanup_module: {
				id: undefined,
				job: undefined,
				count: undefined,
				iterator: undefined,
				flow_jobs: undefined,
				branch_chosen: undefined,
				branchall: undefined,
				approvers: undefined,
				failed_retries: undefined,
				skipped: undefined,
				parent_module: undefined
			}
		},
		raw_flow: {
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
		is_flow_step: false,
		language: undefined,
		is_skipped: false,
		email: 'tutorial@windmill.dev',
		visible_to_owner: true,
		mem_peak: 45,
		tag: 'durable',
		priority: undefined,
		labels: ['tutorial', 'temperature-converter'],
		self_wait_time_ms: 0,
		aggregate_wait_time_ms: 0
	}

	export function runTutorial() {
		// Load the mock job
		onLoad?.(mockJob)
		tutorial?.runTutorial()
	}

	function findTabButton(text: string): HTMLElement | null {
		const buttons = Array.from(document.querySelectorAll('button'))
		return buttons.find((btn) => btn.textContent?.trim() === text) as HTMLElement | null
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={9}
	name="explore-runs"
	tainted={false}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Explore flow execution',
					description:
						"Let's explore a completed flow run and see what information is available about the execution.",
					onNextClick: async () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '[data-testid="result-panel"]',
				onHighlighted: async () => {
					step1Complete = false
					await wait(DELAY_MEDIUM)
					step1Complete = true
				},
				popover: {
					title: 'View the result',
					description:
						'This panel shows the final result of the flow. Our temperature converter returned the temperature in Fahrenheit with a category and emoji.',
					side: 'left',
					onNextClick: () => {
						if (!step1Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '[data-testid="logs-tab"]',
				onHighlighted: async () => {
					step2Complete = false

					await wait(DELAY_MEDIUM)

					const logsTab = findTabButton('Logs')
					if (logsTab) {
						logsTab.click()
						await wait(DELAY_LONG)
					}

					step2Complete = true
				},
				popover: {
					title: 'Check the logs',
					description: 'The logs show detailed execution information for each step of the flow.',
					side: 'bottom',
					onNextClick: () => {
						if (!step2Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			},
			{
				element: '[data-testid="flow-status-viewer"]',
				onHighlighted: async () => {
					step3Complete = false
					await wait(DELAY_MEDIUM)
					step3Complete = true
				},
				popover: {
					title: 'Flow visualization',
					description:
						'This diagram shows all the steps in your flow. You can click on individual steps to see their specific results and logs.',
					side: 'top',
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
				element: '[data-testid="job-metadata"]',
				onHighlighted: async () => {
					step4Complete = false
					await wait(DELAY_MEDIUM)
					step4Complete = true
				},
				popover: {
					title: 'Execution metadata',
					description:
						'Here you can see important details like execution time, who ran it, when it started, and resource usage.',
					side: 'left',
					onNextClick: () => {
						if (!step4Complete) {
							sendUserToast('Please wait...', false, [], undefined, 3000)
							return
						}
						driver.moveNext()
					}
				}
			}
		]

		return steps
	}}
/>

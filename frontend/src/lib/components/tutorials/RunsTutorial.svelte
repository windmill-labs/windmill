<script lang="ts">
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { updateProgress } from '$lib/tutorialUtils'
	import { JobService, FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { wait } from '$lib/utils'
	import { waitJob } from '$lib/components/waitJob'
	import { DELAY_SHORT, DELAY_MEDIUM, DELAY_LONG, DELAY_ANIMATION, createFakeCursor } from './utils'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = $state(undefined)

	// Create a simple flow
	async function createTutorialFlow(): Promise<string> {
		const flowPath = `f/tutorial/runs-tutorial-flow-${Date.now()}`
		const flow: Flow = {
			summary: 'Tutorial: Simple Hello World Flow',
			description: 'A simple flow created for the runs tutorial',
			value: {
				modules: [
					{
						id: 'hello',
						value: {
							type: 'rawscript',
							content: 'export async function main() {\n  return {\n    message: "Hello from the Runs tutorial!",\n    timestamp: new Date().toISOString()\n  };\n}',
							language: 'bun',
							input_transforms: {}
						},
						summary: 'Say hello'
					}
				]
			},
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				type: 'object',
				properties: {},
				required: [],
				order: []
			},
			path: flowPath,
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {}
		}

		await FlowService.createFlow({
			workspace: $workspaceStore!,
			requestBody: flow
		})

		return flowPath
	}

	// Create a broken flow that will fail
	async function createBrokenFlow(): Promise<string> {
		const flowPath = `f/tutorial/runs-tutorial-broken-${Date.now()}`
		const flow: Flow = {
			summary: 'Tutorial: Broken Flow Example',
			description: 'A flow that intentionally fails to demonstrate error handling',
			value: {
				modules: [
					{
						id: 'error',
						value: {
							type: 'rawscript',
							content: 'export async function main() {\n  throw new Error("Intentional error for tutorial - this demonstrates how failed jobs appear in the runs list");\n}',
							language: 'bun',
							input_transforms: {}
						},
						summary: 'Throw error'
					}
				]
			},
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				type: 'object',
				properties: {},
				required: [],
				order: []
			},
			path: flowPath,
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {}
		}

		await FlowService.createFlow({
			workspace: $workspaceStore!,
			requestBody: flow
		})

		return flowPath
	}

	// Run the flow and wait for completion
	async function runFlowAndWait(flowPath: string): Promise<string> {
		const jobId = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path: flowPath,
			requestBody: {},
			skipPreprocessor: true
		})

		// Wait for job to complete
		await waitJob(jobId)
		return jobId
	}

	function getTutorialSteps(driver: any): DriveStep[] {
		let successfulFlowPath: string | undefined = undefined
		
		return [
			{
				popover: {
					title: 'Welcome to the Runs page!',
					description:
						"Let's explore how to monitor and manage your script and flow executions in Windmill. We'll create some example runs to demonstrate the features.",
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				popover: {
					title: 'Creating and running a flow...',
					description:
						'We\'ll create a simple flow, deploy it, and run it so you can see how runs appear in this page.',
					onNextClick: async () => {
						try {
							successfulFlowPath = await createTutorialFlow()
							await wait(DELAY_MEDIUM)
							if (successfulFlowPath) {
								await runFlowAndWait(successfulFlowPath)
								await wait(DELAY_LONG)
							}
							driver.moveNext()
						} catch (error) {
							console.error('Error creating/running flow:', error)
							driver.moveNext()
						}
					}
				}
			},
			{
				element: 'canvas',
				popover: {
					title: 'Visual run history',
					description:
						'This chart gives you a visual overview of your run history at a glance. You can quickly see the pattern of executions over time.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#runs-table-wrapper',
				onHighlighted: async () => {
					await wait(DELAY_MEDIUM)
					// Find the first run row (div with cursor-pointer class inside the wrapper)
					const firstRunRow = document.querySelector(
						'#runs-table-wrapper .cursor-pointer'
					) as HTMLElement
					if (firstRunRow) {
						// Create cursor with initial position
						const cursor = createFakeCursor()
						const rowRect = firstRunRow.getBoundingClientRect()
						
						// Set initial position (off-screen to the left)
						cursor.style.left = `${rowRect.left - 100}px`
						cursor.style.top = `${rowRect.top + rowRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						// Animate to target position
						cursor.style.left = `${rowRect.left + rowRect.width / 2}px`
						cursor.style.top = `${rowRect.top + rowRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						// Click on the run row
						firstRunRow.click()
						await wait(DELAY_SHORT)
						
						// Remove the cursor
						cursor.remove()
						// Wait for navigation to job details page
						await wait(DELAY_LONG)
					}
				},
				popover: {
					title: 'Your execution history',
					description:
						'This table shows all your recent script and flow runs. We\'re clicking on the first run to view its details, logs, and results.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.flex.flex-col.gap-2.items-start.p-4.pb-8.min-h-full',
				onHighlighted: async () => {
					// Wait for the results div to be visible
					await wait(DELAY_MEDIUM)
				},
				popover: {
					title: 'Job results',
					description:
						'Here you can see the results of the job execution, including any output data, return values, and execution details.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				popover: {
					title: 'Creating a broken flow...',
					description:
						'Now let\'s create a flow that will fail, so you can see how failed jobs appear in the runs list.',
					onNextClick: async () => {
						try {
							// Navigate back to runs page first if we're on a job details page
							if (window.location.pathname.includes('/run/')) {
								window.history.back()
								await wait(DELAY_LONG)
							}
							
							const brokenFlowPath = await createBrokenFlow()
							await wait(DELAY_MEDIUM)
							// Run the broken flow
							await runFlowAndWait(brokenFlowPath)
							await wait(DELAY_LONG)
							driver.moveNext()
						} catch (error) {
							console.error('Error creating/running broken flow:', error)
							driver.moveNext()
						}
					}
				}
			},
			{
				element: '#runs-table-wrapper',
				onHighlighted: async () => {
					await wait(DELAY_MEDIUM)
					// Find the failed job (red badge/X icon)
					const failedJobRow = Array.from(
						document.querySelectorAll('#runs-table-wrapper .cursor-pointer')
					).find((el) => {
						// Look for a red badge or X icon indicating failure
						const badge = el.querySelector('[class*="bg-red"], [class*="text-red"]')
						return badge !== null
					}) as HTMLElement

					if (failedJobRow) {
						// Create cursor with initial position
						const cursor = createFakeCursor()
						const rowRect = failedJobRow.getBoundingClientRect()
						
						// Set initial position (off-screen to the left)
						cursor.style.left = `${rowRect.left - 100}px`
						cursor.style.top = `${rowRect.top + rowRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						// Animate to target position
						cursor.style.left = `${rowRect.left + rowRect.width / 2}px`
						cursor.style.top = `${rowRect.top + rowRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						// Click on the failed job row
						failedJobRow.click()
						await wait(DELAY_SHORT)
						
						// Remove the cursor
						cursor.remove()
						// Wait for navigation to job details page
						await wait(DELAY_LONG)
					}
				},
				popover: {
					title: 'Failed jobs',
					description:
						'Failed jobs are marked with a red badge. Click on a failed job to see the error details and understand what went wrong.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.flex.flex-col.gap-2.items-start.p-4.pb-8.min-h-full',
				onHighlighted: async () => {
					// Wait for the error results div to be visible
					await wait(DELAY_MEDIUM)
				},
				popover: {
					title: 'Error details',
					description:
						'Here you can see the error message and details from the failed job. This helps you understand what went wrong and how to fix it.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				popover: {
					title: 'Tutorial complete! 🎉',
					description:
						'You now know how to use the Runs page to monitor your executions, view successful results, and debug failed jobs.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu.</p>',
					onNextClick: () => {
						updateProgress(index)
						driver.destroy()
					}
				}
			}
		]
	}


	// Start tutorial
	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	name="runs-tutorial"
	tainted={false}
	on:error
	on:skipAll
	getSteps={(driver) => {
		return getTutorialSteps(driver)
	}}
/>

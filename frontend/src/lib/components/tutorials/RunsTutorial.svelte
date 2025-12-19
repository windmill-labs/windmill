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
	let jobsExist: boolean | undefined = $state(undefined)

	// Check if jobs exist
	async function checkJobsExist(): Promise<boolean> {
		try {
			const jobs = await JobService.listJobs({
				workspace: $workspaceStore!,
				jobKinds: 'script,flow',
				perPage: 1
			})
			return jobs && jobs.length > 0
		} catch (error) {
			console.error('Error checking jobs:', error)
			return false
		}
	}

	// Create a simple flow for Scenario B
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

	// Scenario A: Jobs exist
	function getScenarioASteps(driver: any): DriveStep[] {
		return [
			{
				popover: {
					title: 'Welcome to the Runs page!',
					description:
						"Let's explore how to monitor and manage your script and flow executions in Windmill.",
					onNextClick: () => {
						driver.moveNext()
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
					title: 'Tutorial complete! 🎉',
					description:
						'You now know how to use the Runs page to monitor your executions.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu.</p>',
					onNextClick: () => {
						updateProgress(index)
						driver.destroy()
					}
				}
			}
		]
	}

	// Scenario B: No jobs exist - create flow, deploy, run it
	function getScenarioBSteps(driver: any): DriveStep[] {
		let flowPath: string | undefined = undefined

		return [
			{
				popover: {
					title: 'Welcome to the Runs page!',
					description:
						"Let's explore how to monitor and manage your script and flow executions. Since you don't have any runs yet, we'll create one together.",
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				popover: {
					title: 'Creating a flow for you...',
					description:
						'We\'ll create a simple flow, deploy it, and run it so you can see how runs appear in this page.',
					onNextClick: async () => {
						try {
							flowPath = await createTutorialFlow()
							await wait(DELAY_MEDIUM)
							driver.moveNext()
						} catch (error) {
							console.error('Error creating flow:', error)
							driver.moveNext()
						}
					}
				}
			},
			{
				popover: {
					title: 'Running the flow...',
					description: 'Now we\'ll execute the flow. Watch as it appears in your runs list!',
					onNextClick: async () => {
						try {
							if (flowPath) {
								await runFlowAndWait(flowPath)
								await wait(DELAY_LONG)
								// Reload the page to see the new run
								window.location.reload()
							} else {
								driver.moveNext()
							}
						} catch (error) {
							console.error('Error running flow:', error)
							driver.moveNext()
						}
					}
				}
			},
			{
				element: '#runs-table-wrapper',
				popover: {
					title: 'Your execution history',
					description:
						'Here you can see the run we just created! This table shows all your script and flow runs with their status, duration, and results.',
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
						'You now know how to use the Runs page to monitor your executions.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu.</p>',
					onNextClick: () => {
						updateProgress(index)
						driver.destroy()
					}
				}
			}
		]
	}

	// Start tutorial
	export async function runTutorial() {
		// Check if jobs exist before starting tutorial
		jobsExist = await checkJobsExist()
		// Small delay to ensure state is updated
		await wait(100)
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
		// Use the pre-checked jobsExist value to determine scenario
		if (jobsExist === undefined) {
			// Fallback to Scenario A if check hasn't completed
			return getScenarioASteps(driver)
		}

		if (jobsExist) {
			// Scenario A: Jobs exist - guide user to click on a run
			return getScenarioASteps(driver)
		} else {
			// Scenario B: No jobs - create flow, deploy, run it
			return getScenarioBSteps(driver)
		}
	}}
/>

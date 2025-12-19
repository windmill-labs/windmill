<script lang="ts">
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { updateProgress } from '$lib/tutorialUtils'
	import { JobService, FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { wait } from '$lib/utils'
	import { waitJob } from '$lib/components/waitJob'
	import { DELAY_SHORT, DELAY_MEDIUM, DELAY_LONG, DELAY_ANIMATION, createFakeCursor } from './utils'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = $state(undefined)
	let tutorialFlowPaths: string[] = $state([])

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
		return [
			{
				popover: {
					title: 'Welcome to your monitoring dashboard!',
					description:
						"Let's explore how to monitor and manage your script and flow executions in Windmill. We've created some example runs for you.",
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: '#runs-table-wrapper',
				onHighlighted: async () => {
					await wait(DELAY_MEDIUM)
					
					// Find both jobs
					const allJobRows = Array.from(
						document.querySelectorAll('#runs-table-wrapper .cursor-pointer')
					) as HTMLElement[]
					
					// Find successful job (green badge/check icon) - first one that's not failed
					const successfulJobRow = allJobRows.find((el) => {
						const hasRedBadge = el.querySelector('[class*="bg-red"], [class*="text-red"]')
						const hasGreenBadge = el.querySelector('[class*="bg-green"], [class*="text-green"]')
						return !hasRedBadge && hasGreenBadge !== null
					}) || allJobRows[0]

					// Find failed job (red badge/X icon)
					const failedJobRow = allJobRows.find((el) => {
						const badge = el.querySelector('[class*="bg-red"], [class*="text-red"]')
						return badge !== null
					})

					if (successfulJobRow && failedJobRow) {
						// Create cursor once for both clicks
						const cursor = createFakeCursor()
						
						// Click on successful job first
						const successRect = successfulJobRow.getBoundingClientRect()
						cursor.style.left = `${successRect.left - 100}px`
						cursor.style.top = `${successRect.top + successRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						cursor.style.left = `${successRect.left + successRect.width / 2}px`
						cursor.style.top = `${successRect.top + successRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						successfulJobRow.click()
						await wait(DELAY_SHORT)
						
						// Wait for navigation to job details page
						await wait(DELAY_LONG)
						
						// Navigate back to runs page using SvelteKit navigation
						await goto(`${base}/runs?tutorial=runs-tutorial`, { replaceState: true })
						await wait(DELAY_LONG)
						
						// Re-find the failed job after navigation
						const failedJobRowAfterNav = Array.from(
							document.querySelectorAll('#runs-table-wrapper .cursor-pointer')
						).find((el) => {
							const badge = el.querySelector('[class*="bg-red"], [class*="text-red"]')
							return badge !== null
						}) as HTMLElement
						
						if (failedJobRowAfterNav) {
							// Click on failed job
							const failedRect = failedJobRowAfterNav.getBoundingClientRect()
							cursor.style.left = `${failedRect.left - 100}px`
							cursor.style.top = `${failedRect.top + failedRect.height / 2}px`
							await wait(DELAY_SHORT)
							
							cursor.style.left = `${failedRect.left + failedRect.width / 2}px`
							cursor.style.top = `${failedRect.top + failedRect.height / 2}px`
							await wait(DELAY_ANIMATION)
							await wait(DELAY_MEDIUM)
							
							failedJobRowAfterNav.click()
							await wait(DELAY_SHORT)
							
							// Wait for navigation to job details page
							await wait(DELAY_LONG)
							
							// Navigate back to runs page using SvelteKit navigation
							await goto(`${base}/runs?tutorial=runs-tutorial`, { replaceState: true })
							await wait(DELAY_LONG)
						}
						
						// Remove the cursor
						cursor.remove()
					}
				},
				popover: {
					title: 'Exploring job runs. 1 success ✅ and 1 failure ❌',
					description:
						'We\'re clicking on both jobs to show you how to inspect different types of executions.',
					side: 'bottom',
					onNextClick: async () => {
						// Find and click the failed job again (last one clicked) without showing cursor
						const failedJobRow = Array.from(
							document.querySelectorAll('#runs-table-wrapper .cursor-pointer')
						).find((el) => {
							const badge = el.querySelector('[class*="bg-red"], [class*="text-red"]')
							return badge !== null
						}) as HTMLElement
						
						if (failedJobRow) {
							failedJobRow.click()
							await wait(DELAY_SHORT)
							
							// Wait for navigation to job details page
							await wait(DELAY_LONG)
							
							// Navigate back to runs page using SvelteKit navigation
							await goto(`${base}/runs?tutorial=runs-tutorial`, { replaceState: true })
							await wait(DELAY_LONG)
						}
						
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.p-2.px-4.pt-8.w-full.border-b',
				popover: {
					title: 'Visual run history',
					description:
						'This chart gives you a visual overview of your run history at a glance. The duration chart shows how long each job takes to complete over time.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.p-2.px-4.pt-8.w-full.border-b',
				onHighlighted: async () => {
					await wait(DELAY_MEDIUM)
					// Find the button with data-value="ConcurrencyChart"
					const concurrencyButton = document.querySelector(
						'button[data-value="ConcurrencyChart"]'
					) as HTMLElement
					
					if (concurrencyButton) {
						// Create cursor with initial position
						const cursor = createFakeCursor()
						const buttonRect = concurrencyButton.getBoundingClientRect()
						
						// Set initial position (off-screen to the left)
						cursor.style.left = `${buttonRect.left - 100}px`
						cursor.style.top = `${buttonRect.top + buttonRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						// Animate to target position
						cursor.style.left = `${buttonRect.left + buttonRect.width / 2}px`
						cursor.style.top = `${buttonRect.top + buttonRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						// Click on the button
						concurrencyButton.click()
						await wait(DELAY_SHORT)
						
						// Remove the cursor
						cursor.remove()
						await wait(DELAY_MEDIUM)
					}
				},
				popover: {
					title: 'Switching chart views',
					description:
						'You can switch between different chart views to analyze your runs. The concurrency chart allows you to see how many jobs are running concurrently over time.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.flex.flex-row.items-start.w-full.border-b.px-4.gap-8',
				onHighlighted: async () => {
					await wait(DELAY_MEDIUM)
					
					// Find the success and failure filter buttons
					const successButton = document.querySelector(
						'button[data-value="success"]'
					) as HTMLElement
					const failureButton = document.querySelector(
						'button[data-value="failure"]'
					) as HTMLElement
					
					if (successButton && failureButton) {
						// Create cursor once for both clicks
						const cursor = createFakeCursor()
						
						// Click on success button first
						const successRect = successButton.getBoundingClientRect()
						cursor.style.left = `${successRect.left - 100}px`
						cursor.style.top = `${successRect.top + successRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						cursor.style.left = `${successRect.left + successRect.width / 2}px`
						cursor.style.top = `${successRect.top + successRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						successButton.click()
						await wait(DELAY_SHORT)
						await wait(DELAY_MEDIUM)
						
						// Click on failure button
						const failureRect = failureButton.getBoundingClientRect()
						cursor.style.left = `${failureRect.left - 100}px`
						cursor.style.top = `${failureRect.top + failureRect.height / 2}px`
						await wait(DELAY_SHORT)
						
						cursor.style.left = `${failureRect.left + failureRect.width / 2}px`
						cursor.style.top = `${failureRect.top + failureRect.height / 2}px`
						await wait(DELAY_ANIMATION)
						await wait(DELAY_MEDIUM)
						
						failureButton.click()
						await wait(DELAY_SHORT)
						
						// Remove the cursor
						cursor.remove()
						await wait(DELAY_MEDIUM)
					}
				},
				popover: {
					title: 'Filtering jobs date, kind, status',
					description:
						'You can filter jobs, for example by status (failed, running, success). This helps you focus on specific types of executions.',
					side: 'bottom',
					onNextClick: () => {
						driver.moveNext()
					}
				}
			},
			{
				element: 'div.flex.flex-row.gap-4.items-center.px-2.py-1.grow-0.justify-between',
				popover: {
					title: 'More filtering options',
					description:
						'Even more filters are available to help you find exactly what you\'re looking for. Explore the additional filtering options to refine your search.',
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
					onNextClick: async () => {
						updateProgress(index)
						driver.destroy()
						// Cleanup tutorial flows silently
						await cleanupTutorialFlows()
					}
				}
			}
		]
	}


	// Cleanup function to delete tutorial flows
	async function cleanupTutorialFlows() {
		for (const flowPath of tutorialFlowPaths) {
			try {
				await FlowService.deleteFlowByPath({
					workspace: $workspaceStore!,
					path: flowPath
				})
			} catch (error) {
				console.error(`Error deleting tutorial flow ${flowPath}:`, error)
			}
		}
	}

	// Start tutorial - create and run both jobs first
	export async function runTutorial() {
		// Create and run both flows at the beginning
		try {
			const successfulFlowPath = await createTutorialFlow()
			const brokenFlowPath = await createBrokenFlow()
			
			// Store flow paths for cleanup
			tutorialFlowPaths = [successfulFlowPath, brokenFlowPath]
			
			// Run both flows in parallel
			await Promise.all([
				runFlowAndWait(successfulFlowPath),
				runFlowAndWait(brokenFlowPath)
			])
			
			// Wait a bit for jobs to appear
			await wait(DELAY_LONG)
		} catch (error) {
			console.error('Error creating/running tutorial flows:', error)
		}
		
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

<script lang="ts">
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { updateProgress } from '$lib/tutorialUtils'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = $state(undefined)

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
		const steps: DriveStep[] = [
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
				popover: {
					title: 'Your execution history',
					description:
						'This table shows all your recent script and flow runs. You can see the status, duration, and results of each execution.',
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

		return steps
	}}
/>

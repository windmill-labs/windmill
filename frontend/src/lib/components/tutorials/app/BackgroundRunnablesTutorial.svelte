<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import { type DriveStep } from 'driver.js'
	import Tutorial from '../Tutorial.svelte'
	import { clickButtonBySelector } from '../utils'

	export let name: string
	export let index: number

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial(skipStepsCount: number | undefined = undefined) {
		tutorial?.runTutorial({ skipStepsCount })
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	on:error
	on:skipAll
	getSteps={(driver, options) => {
		const steps: DriveStep[] = [
			{
				element: '#app-editor-runnable-panel',
				popover: {
					title: 'Runnable panel',
					description:
						'This is the runnable panel. Here you can add runnables to your app. Runnables are scripts that can be executed in the background. You can add as many runnables as you want.'
				}
			},
			{
				element: '#create-background-runnable',
				popover: {
					title: 'Create a runnable',
					description:
						'Click here to create a runnable. Runnables are scripts that can be executed in the background. You can add as many runnables as you want.',
					onNextClick: () => {
						clickButtonBySelector('#create-background-runnable')
						setTimeout(() => driver.moveNext())
					}
				}
			},
			{
				element: '#app-editor-empty-runnable',
				popover: {
					title: 'Empty runnable panel',
					description:
						'This is the empty runnable panel. Here you can add runnables to your app. Runnables are scripts that can be executed in the background. You can add as many runnables as you want. You can also select a script or a flow from your workspace or the Hub.'
				}
			},

			{
				element: '#app-editor-backend-runnables',
				popover: {
					title: 'Backend runnables',
					description:
						'Backend runnables are scripts that are executed on the server. They can be used to perform tasks that are not possible to be performed on the client. For example, you can use backend runnables to send emails, perform database operations, etc.'
				}
			},
			{
				element: '#app-editor-frontend-runnables',
				popover: {
					title: 'Frontend runnables',
					description:
						'Frontend scripts are executed in the browser and can manipulate the app context directly. You can also interact with components using component controls.',
					onNextClick: () => {
						setTimeout(() => {
							driver.moveNext()

							updateProgress(5)
						})
					}
				}
			}
		]

		// Remove steps if we want to skip them (excpet the first one)

		if (options?.skipStepsCount) {
			steps.splice(1, options.skipStepsCount)
		}

		return steps
	}}
/>

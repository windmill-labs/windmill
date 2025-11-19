<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { base } from '$lib/base'

	export let name: string = 'workspace-onboarding'
	export let index: number = 8

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	on:error
	on:skipAll
	tainted={false}
	getSteps={(driver) => {
		// Helper function to find the create script button
		const findScriptButton = (): HTMLElement | null => {
			const button = document.querySelector('#create-script-button') as HTMLElement | null
			if (button) {
				console.log('Found script button:', button)
			} else {
				console.error('Could not find script button')
			}
			return button
		}

		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to your Windmill workspace! 🎉',
					description:
						"Let's take a quick tour! In this tutorial, we'll create a simple flow so you",
					onNextClick: () => {
						// Wait a bit to ensure the page is fully rendered before moving to next step
						setTimeout(() => {
							const button = findScriptButton()
							if (button) {
								driver.moveNext()
							} else {
								alert('Could not find the Create Script button. Please make sure you are on the home page.')
							}
						}, 100)
					}
				}
			},
			{
				popover: {
					title: 'Create your first script',
					description:
						'Click to create your first script!',
					onNextClick: async () => {
						// Mark tutorial as complete
						updateProgress(index)
						driver.destroy()
					}
				},
				element: '#create-script-button',
			}
		]

		return steps
	}}
/>

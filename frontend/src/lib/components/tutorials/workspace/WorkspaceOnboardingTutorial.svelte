<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { base } from '$lib/base'

	let { name = 'workspace-onboarding', index = 8, onerror, onskipAll } = $props()

	let tutorial: Tutorial | undefined = $state(undefined)

	function hideOverlay() {
		const overlay = document.querySelector('.driver-overlay') as HTMLElement
		if (overlay) {
			overlay.style.display = 'none'
		}
	}

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	{onerror}
	onskipAll={onskipAll}
	tainted={false}
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to your Windmill workspace! 🎉',
					description:
						"Let's take a quick tour! In this tutorial, we'll create a simple flow so you",
					onNextClick: () => {
						// Wait a bit to ensure the page is fully rendered before moving to next step
						setTimeout(() => {
							const button = document.querySelector('#create-script-button') as HTMLElement | null
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
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Click to create your first script!</p>',
					onNextClick: async () => {
						// Move to the next step (Create Flow button)
						setTimeout(() => {
							const button = document.querySelector('#create-flow-button') as HTMLElement | null
							if (button) {
								driver.moveNext()
							} else {
								alert('Could not find the Create Flow button. Please make sure you are on the home page.')
							}
						}, 100)
					}
				},
				element: '#create-script-button',
			},
			{
				popover: {
					title: 'Create your first flow',
					description:
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Discover together how flows work in Windmill</p>',
					onNextClick: async () => {
						// Move to the next step (Create App button)
						setTimeout(() => {
							const button = document.querySelector('#create-app-button') as HTMLElement | null
							if (button) {
								driver.moveNext()
							} else {
								alert('Could not find the Create App button. Please make sure you are on the home page.')
							}
						}, 100)
					}
				},
				element: '#create-flow-button',
			},
			{
				popover: {
					title: 'Create your first app',
					description:
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Build low-code applications with Windmill</p>',
					onNextClick: async () => {
						// Clear any existing flow drafts from localStorage to ensure fresh start
						try {
							localStorage.removeItem('flow')
						} catch (e) {
							console.error('Error clearing localStorage', e)
						}
						// Hide overlay before navigation
						hideOverlay()
						// Mark tutorial as complete
						updateProgress(index)
						driver.destroy()
						// Navigate to the flow creation page with tutorial continuation parameter and nodraft
						window.location.href = `${base}/flows/add?tutorial=workspace-onboarding-continue&nodraft=true`
					}
				},
				element: '#create-app-button',
			}
		]

		return steps
	}}
/>

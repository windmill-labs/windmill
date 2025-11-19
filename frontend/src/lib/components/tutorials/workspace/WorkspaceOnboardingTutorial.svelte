<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { base } from '$lib/base'

	export let name: string = 'workspace-onboarding'
	export let index: number = 8

	let tutorial: Tutorial | undefined = undefined

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

		// Helper function to find the create flow button
		const findFlowButton = (): HTMLElement | null => {
			const button = document.querySelector('#create-flow-button') as HTMLElement | null
			if (button) {
				console.log('Found flow button:', button)
			} else {
				console.error('Could not find flow button')
			}
			return button
		}

		// Helper function to find the create app button
		const findAppButton = (): HTMLElement | null => {
			const button = document.querySelector('#create-app-button') as HTMLElement | null
			if (button) {
				console.log('Found app button:', button)
			} else {
				console.error('Could not find app button')
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
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Click to create your first script!</p>',
					onNextClick: async () => {
						// Move to the next step (Create Flow button)
						setTimeout(() => {
							const button = findFlowButton()
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
							const button = findAppButton()
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

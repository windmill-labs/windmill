<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = $state(undefined)

	export function runTutorial() {
		// Check if we're on the homepage
		if ($page.url.pathname !== `${base}/` && $page.url.pathname !== `${base}`) {
			// Redirect to homepage with a tutorial parameter
			goto(`${base}/?tutorial=workspace-onboarding`)
		} else {
			tutorial?.runTutorial()
		}
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={index}
	name="workspace-onboarding"
	tainted={false}
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to your Windmill workspace! 🎉',
					description:
						"Let's take a quick tour! We will show you the main sections of your workspace.",
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
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>Scripts turn code into tools. Write in Python, TypeScript, Go, Bash, SQL and more. Run them manually, on schedule, or via webhooks.</p>',
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
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>Flows orchestrate multiple scripts. Chain them together with branching, loops, and error handling to build complex workflows.</p>',
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
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>Apps are custom UIs built with drag-and-drop. Combine tables, forms, charts, and buttons that trigger your scripts and flows.. That\'s it for the tour!</p><p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu or in the <strong>Help</strong> submenu.</p>',
					onNextClick: async () => {
						// Mark tutorial as complete
						updateProgress(index)
						driver.destroy()

						// Clean up URL parameter if present
						if ($page.url.searchParams.has('tutorial')) {
							goto(`${base}/`, { replaceState: true })
						}
					}
				},
				element: '#create-app-button',
			}
		]

		return steps
	}}
/>

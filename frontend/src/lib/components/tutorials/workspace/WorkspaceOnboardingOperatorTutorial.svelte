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
			goto(`${base}/?tutorial=workspace-onboarding-operator`)
		} else {
			tutorial?.runTutorial()
		}
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	name="workspace-onboarding-operator"
	tainted={false}
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to your Windmill workspace! 🎉',
					description:
						"Let's take a quick tour! We'll show you how to view and run the main resources in Windmill: Scripts, Flows, and Apps.",
					onNextClick: () => {
						// Wait a bit to ensure the page is fully rendered before moving to next step
						setTimeout(() => {
							// Try to find scripts navigation or section
							const scriptsLink = document.querySelector('a[href*="/scripts"]') as HTMLElement | null
							const scriptsSection = document.querySelector('[data-section="scripts"]') as HTMLElement | null
							const scriptsButton = document.querySelector('#view-scripts-button, #scripts-nav, .scripts-link') as HTMLElement | null

							if (scriptsLink || scriptsSection || scriptsButton) {
								driver.moveNext()
							} else {
								// If we can't find specific elements, just move to next step
								driver.moveNext()
							}
						}, 100)
					}
				}
			},
			{
				popover: {
					title: 'Scripts - Your automation tools',
					description:
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Scripts</strong> are the building blocks of automation in Windmill. They can be written in Python, TypeScript, Go, Bash, SQL and more.</p><p style="margin-top: 8px;">As an operator, you can <strong>view</strong> existing scripts and <strong>run them</strong> with different parameters to perform various tasks.</p>',
					onNextClick: async () => {
						// Move to the next step (Flows)
						setTimeout(() => {
							const flowsLink = document.querySelector('a[href*="/flows"]') as HTMLElement | null
							const flowsSection = document.querySelector('[data-section="flows"]') as HTMLElement | null

							if (flowsLink || flowsSection) {
								driver.moveNext()
							} else {
								driver.moveNext()
							}
						}, 100)
					}
				},
				element: 'a[href*="/scripts"], [data-section="scripts"], #view-scripts-button, #scripts-nav, .scripts-link'
			},
			{
				popover: {
					title: 'Flows - Multi-step workflows',
					description:
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Flows</strong> are multi-step workflows that orchestrate multiple scripts together. They can include branching logic, loops, and error handling.</p><p style="margin-top: 8px;">As an operator, you can <strong>execute flows</strong> to run complex automation sequences and <strong>monitor their progress</strong> in real-time.</p>',
					onNextClick: async () => {
						// Move to the next step (Apps)
						setTimeout(() => {
							const appsLink = document.querySelector('a[href*="/apps"]') as HTMLElement | null
							const appsSection = document.querySelector('[data-section="apps"]') as HTMLElement | null

							if (appsLink || appsSection) {
								driver.moveNext()
							} else {
								driver.moveNext()
							}
						}, 100)
					}
				},
				element: 'a[href*="/flows"], [data-section="flows"], #view-flows-button, #flows-nav, .flows-link'
			},
			{
				popover: {
					title: 'Apps - Custom user interfaces',
					description:
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Apps</strong> are custom user interfaces built with drag-and-drop components. They can include tables, forms, charts, and buttons that trigger scripts and flows.</p><p style="margin-top: 8px;">As an operator, you can <strong>use apps</strong> to interact with data and trigger automations through intuitive interfaces built specifically for your needs.</p><p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>🎉 That\'s it for the tour!</strong></p><p style="margin-top: 8px; font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> You can always access the documentation and support resources from the help menu.</p>',
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
				element: 'a[href*="/apps"], [data-section="apps"], #view-apps-button, #apps-nav, .apps-link'
			}
		]

		return steps
	}}
/>

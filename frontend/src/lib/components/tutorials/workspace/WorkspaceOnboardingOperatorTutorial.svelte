<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'
	import { wait } from '$lib/utils'
	import { DELAY_MEDIUM } from '../utils'

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
					title: 'Welcome to Windmill! 🎉',
					description:
						"Let's take a quick tour! We'll show you the three main tools you can use: Scripts, Flows, and Apps.",
					onNextClick: () => {
						// Wait a bit to ensure the page is fully rendered before moving to next step
						setTimeout(() => {
							// Try to find the script tab button
							const scriptsButton = document.querySelector('[data-value="script"]') as HTMLElement | null

							if (scriptsButton) {
								driver.moveNext()
							} else {
								// If we can't find the button, just move to next step anyway
								driver.moveNext()
							}
						}, 100)
					}
				}
			},
			{
				popover: {
					title: 'Scripts - Run automated tasks',
					description:
						'<img src="/script-tutorial-operator.png" alt="Script Example" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Scripts</strong> are ready-to-use tasks that do things automatically for you.</p><p style="margin-top: 8px;">You can <strong>run scripts</strong> whenever you need them - like generating a report, sending notifications, or processing data.</p>',
					onNextClick: async () => {
						// Move to the next step (Flows)
						setTimeout(() => {
							const flowsButton = document.querySelector('[data-value="flow"]') as HTMLElement | null

							if (flowsButton) {
								driver.moveNext()
							} else {
								driver.moveNext()
							}
						}, 100)
					}
				},
				element: '[data-value="script"]'
			},
			{
				popover: {
					title: 'Flows - Run step-by-step processes',
					description:
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Flows</strong> are processes that run multiple tasks in order, one after another.</p><p style="margin-top: 8px;">You can <strong>start a flow</strong> and watch it complete each step automatically - perfect for tasks that have multiple stages.</p>',
					onNextClick: async () => {
						// Move to the next step (Apps)
						setTimeout(() => {
							const appsButton = document.querySelector('[data-value="app"]') as HTMLElement | null

							if (appsButton) {
								driver.moveNext()
							} else {
								driver.moveNext()
							}
						}, 100)
					}
				},
				element: '[data-value="flow"]'
			},
			{
				popover: {
					title: 'Apps - Use custom tools',
					description:
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Apps</strong> are easy-to-use tools with buttons, forms, and displays built just for your team.</p><p style="margin-top: 8px;">You can <strong>open an app</strong> to work with your data, fill out forms, or trigger tasks - no technical knowledge needed!</p>',
					onNextClick: async () => {
						// Move to the next step (cursor animation)
						driver.moveNext()
					}
				},
				element: '[data-value="app"]'
			},
			{
				popover: {
					title: 'Finally, the Menu section',
					description: 'Explore available tabs where you can access your history of runs, your scheduled scripts, your tutorials progress etc.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu.</p>',
					onNextClick: async () => {
						// Find the target button and click it
						const targetButton = document.querySelector('[role="menuitem"]') as HTMLElement | null
						if (targetButton) {
							targetButton.click()
						}

						// Wait for menu to open
						await wait(DELAY_MEDIUM)

						// Mark tutorial as complete
						updateProgress(index)
						driver.destroy()

						// Clean up URL parameter if present
						if ($page.url.searchParams.has('tutorial')) {
							goto(`${base}/`, { replaceState: true })
						}
					}
				},
				element: '[role="menuitem"]'
			}
		]

		return steps
	}}
/>

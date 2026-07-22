<script lang="ts">
	import { updateProgress } from '$lib/tutorialUtils'
	import Tutorial from '../Tutorial.svelte'
	import type { DriveStep } from 'driver.js'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/state'

	interface Props {
		index: number
	}

	let { index }: Props = $props()

	let tutorial: Tutorial | undefined = $state(undefined)

	export function runTutorial() {
		// Check if we're on the homepage
		if (page.url.pathname !== `${base}/` && page.url.pathname !== `${base}`) {
			// Redirect to homepage with a tutorial parameter
			goto(`${base}/?tutorial=workspace-onboarding`)
		} else {
			tutorial?.runTutorial()
		}
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
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
					onNextClick: async () => {
						// The New menu button mounts once an async permission check resolves, so
						// wait for it before highlighting it in the next step.
						for (let i = 0; i < 20 && !document.querySelector('#create-new-button'); i++) {
							await new Promise((resolve) => setTimeout(resolve, 100))
						}
						driver.moveNext()
					}
				}
			},
			{
				popover: {
					title: 'Create your first script',
					description:
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>Open the <strong>New</strong> menu to create a script. Scripts turn code into tools. Write in Python, TypeScript, Go, Bash, SQL and more. Run them manually, on schedule, or via webhooks.</p>',
					onNextClick: () => {
						driver.moveNext()
					}
				},
				element: '#create-new-button'
			},
			{
				popover: {
					title: 'Create your first flow',
					description:
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>The same <strong>New</strong> menu lets you create a flow. Flows orchestrate multiple scripts. Chain them together with branching, loops, and error handling to build complex workflows.</p>',
					onNextClick: () => {
						driver.moveNext()
					}
				},
				element: '#create-new-button'
			},
			{
				popover: {
					title: 'Create your first app',
					description:
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p>And from the <strong>New</strong> menu you can also create an app. Apps are custom UIs built with drag-and-drop. Combine tables, forms, charts, and buttons that trigger your scripts and flows. That\'s it for the tour!</p><p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to learn more?</strong> Access more tutorials from the <strong>Tutorials</strong> page in the main menu or in the <strong>Help</strong> submenu.</p>',
					onNextClick: async () => {
						// Mark tutorial as complete
						updateProgress(index)
						driver.destroy()

						// Clean up URL parameter if present
						if (page.url.searchParams.has('tutorial')) {
							goto(`${base}/`, { replaceState: true })
						}
					}
				},
				element: '#create-new-button'
			}
		]

		return steps
	}}
/>

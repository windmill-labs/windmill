<script lang="ts">
	import type { DriveStep } from 'driver.js'
	import { wait } from '$lib/utils'
	import Tutorial from './Tutorial.svelte'
	import { markOperatorTourSeen, MENU_OPEN_DELAY_MS } from './operatorTour'

	let tutorial: Tutorial | undefined = $state(undefined)
	let running = false

	export function runTutorial() {
		// A second driver mounted over a live one leaves an overlay that nothing closes.
		if (running) return
		running = true
		tutorial?.runTutorial()
	}

	// Recorded however the tour ends, not only on the last step: someone who closes it has
	// answered the question, and the sidebar entry is how they get it back.
	function onDestroyed() {
		running = false
		void markOperatorTourSeen()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{onDestroyed}
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Welcome to Windmill! 🎉',
					description:
						"Let's take a quick tour! We'll show you the three main tools you can use: Scripts, Flows, and Apps."
				}
			},
			{
				popover: {
					title: 'Scripts - Run automated tasks',
					description:
						'<img src="/script-tutorial-operator.png" alt="Script Example" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Scripts</strong> are ready-to-use tasks that do things automatically for you.</p><p style="margin-top: 8px;">You can <strong>run scripts</strong> whenever you need them - like generating a report, sending notifications, or processing data.</p>'
				},
				element: '[data-value="script"]'
			},
			{
				popover: {
					title: 'Flows - Run step-by-step processes',
					description:
						'<img src="/flow.png" alt="Flow" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Flows</strong> are processes that run multiple tasks in order, one after another.</p><p style="margin-top: 8px;">You can <strong>start a flow</strong> and watch it complete each step automatically - perfect for tasks that have multiple stages.</p>'
				},
				element: '[data-value="flow"]'
			},
			{
				popover: {
					title: 'Apps - Use custom tools',
					description:
						'<img src="/app.png" alt="App" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px; display: block; margin-left: auto; margin-right: auto;" /><p><strong>Apps</strong> are easy-to-use tools with buttons, forms, and displays built just for your team.</p><p style="margin-top: 8px;">You can <strong>open an app</strong> to work with your data, fill out forms, or trigger tasks - no technical knowledge needed!</p>'
				},
				element: '[data-value="app"]'
			},
			{
				popover: {
					title: 'Finally, the Menu section',
					description:
						'Explore available tabs where you can access your history of runs, your scheduled scripts, and your workspaces.<p style="margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(128,128,128,0.3); font-size: 0.9em; opacity: 0.9;"><strong>💡 Want to see this again?</strong> Pick <strong>Take the tour</strong> from that same menu.</p>',
					onNextClick: async () => {
						// The step points into the menu, so it has to be open before the popover
						// lands on it — and open is also where the entry to re-run the tour is.
						const menuButton = document.querySelector('[role="menuitem"]') as HTMLElement | null
						menuButton?.click()
						await wait(MENU_OPEN_DELAY_MS)
						driver.destroy()
					}
				},
				element: '[role="menuitem"]'
			}
		]

		return steps
	}}
/>

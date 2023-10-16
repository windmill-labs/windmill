<script lang="ts">
	import { insertNewGridItem, appComponentFromType } from '$lib/components/apps/editor/appUtils'
	import type { AppComponent } from '$lib/components/apps/editor/component'
	import type { AppViewerContext, AppEditorContext } from '$lib/components/apps/types'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { push } from '$lib/history'
	import { getContext } from 'svelte'
	import Tutorial from '../Tutorial.svelte'
	import { clickButtonBySelector } from '../utils'
	import { updateProgress } from '$lib/tutorialUtils'

	export let name: string
	export let index: number

	let tutorial: Tutorial | undefined = undefined

	const { app, selectedComponent, focusedGrid } = getContext<AppViewerContext>('AppViewerContext')
	const { history } = getContext<AppEditorContext>('AppEditorContext')

	export function runTutorial() {
		tutorial?.runTutorial()
	}

	function addComponent(): void {
		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem(
			$app,
			appComponentFromType('textcomponent') as (id: string) => AppComponent,
			$focusedGrid
		)

		$selectedComponent = [id]
		$app = $app
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	on:error
	on:skipAll
	getSteps={(driver) => [
		{
			popover: {
				title: 'Welcome to the Windmil Flow editor',
				description:
					'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate',
				onNextClick: () => {
					addComponent()
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: `#component-input`,
			popover: {
				title: 'Let’s add a text component',
				description: 'We will connect the input of the text component to an output.'
			}
		},
		{
			element: '#plug',
			popover: {
				title: 'Connect the text component',
				description: 'Click on the plug icon to connect the text component',
				onNextClick: () => {
					clickButtonBySelector('#plug')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#output-ctx',
			popover: {
				title: 'Select the output',
				description:
					"You can now select the output in the output menu. Let's select your email in the app context",
				onNextClick: () => {
					clickButtonBySelector('#output-ctx')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '.val',
			popover: {
				title: 'Click on the output',
				description: 'Simply click on the output to connect it',
				onNextClick: () => {
					clickButtonBySelector('.val')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			popover: {
				title: 'Connection done',
				description: 'You can now see the email output connected to the text component input',
				onNextClick: () => {
					updateProgress(6)

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		}
	]}
/>

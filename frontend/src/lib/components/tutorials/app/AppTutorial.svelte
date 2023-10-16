<script lang="ts">
	import { insertNewGridItem, appComponentFromType } from '$lib/components/apps/editor/appUtils'
	import type { AppComponent, TypedComponent } from '$lib/components/apps/editor/component'
	import type { AppViewerContext, AppEditorContext } from '$lib/components/apps/types'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { push } from '$lib/history'
	import { getContext } from 'svelte'
	import Tutorial from '../Tutorial.svelte'
	import {
		clickButtonBySelector,
		componentComponentSourceToOutput,
		updateBackgroundRunnableCode
	} from '../utils'
	import { updateProgress } from '$lib/tutorialUtils'

	export let name: string
	export let index: number

	let tutorial: Tutorial | undefined = undefined

	const { app, selectedComponent, focusedGrid } = getContext<AppViewerContext>('AppViewerContext')
	const { history } = getContext<AppEditorContext>('AppEditorContext')

	export function runTutorial() {
		tutorial?.runTutorial()
	}

	function addComponent(appComponentType: TypedComponent['type']): void {
		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem(
			$app,
			appComponentFromType(appComponentType) as (id: string) => AppComponent,
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
				title: 'Welcome to the App editor tutorial !',
				description:
					'This tutorial will show you how to use the App editor, add components, background scripts and connect them.',
				onNextClick: () => {
					addComponent('textinputcomponent')
					addComponent('displaycomponent')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '.wm-app-viewer',
			popover: {
				title: 'App canvas',
				description:
					'In the canvas, you can move components around, resize them, and organize them in grids. In this example, we already added a text input and a display component.',
				onNextClick: () => {
					driver.moveNext()
				}
			}
		},
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
				description: 'Click here to create a runnable.',
				onNextClick: () => {
					clickButtonBySelector('#create-background-runnable')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#app-editor-empty-runnable',
			popover: {
				title: 'Choose a language',
				description:
					'You can choose the language of your runnable. For this tutorial, we will use deno.'
			}
		},

		{
			element: '#create-deno-script',
			popover: {
				title: 'Create a deno script',
				description:
					"Let's create a simple deno script. For the sake of this tutorial, we will create a script that converts the text to uppercase.",
				onNextClick: () => {
					clickButtonBySelector('#create-deno-script')
					setTimeout(() => {
						updateBackgroundRunnableCode(
							$app,
							0,
							`export async function main(x: string) {
  return x?.toLocaleUpperCase();
}
`
						)

						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#schema-plug',
			popover: {
				title: 'Connect the function input',
				description:
					"The function we created has an string input 'x'. We can connect the output of the text component to it.",
				onNextClick: () => {
					clickButtonBySelector('#schema-plug')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},

		{
			element: '#output-a',
			popover: {
				title: 'Select the output',
				description:
					"You can now select the output in the output menu. Let's select the result of the text component.",
				onNextClick: () => {
					clickButtonBySelector('#output-a')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element:
				'	div.bg-surface:nth-child(2) > div:nth-child(1) > div:nth-child(2) > div:nth-child(2) > div:nth-child(2) > div:nth-child(2) > div:nth-child(1) > span:nth-child(1) > ul:nth-child(1) > li:nth-child(1) > button:nth-child(2)',
			popover: {
				title: 'Click on the output',
				description: 'Simply click on the output to connect it',
				onNextClick: () => {
					clickButtonBySelector(
						'	div.bg-surface:nth-child(2) > div:nth-child(1) > div:nth-child(2) > div:nth-child(2) > div:nth-child(2) > div:nth-child(2) > div:nth-child(1) > span:nth-child(1) > ul:nth-child(1) > li:nth-child(1) > button:nth-child(2)'
					)
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			popover: {
				title: 'Connect the result of the background runnable',
				description:
					'In the same way, you can connect the result of the background runnable to the display component',
				onNextClick: () => {
					componentComponentSourceToOutput($app, 'b', 'bg_0')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '.wm-app-viewer',
			popover: {
				title: "Let's test out the app !",
				description:
					'We can now type in the text input and see the result in the display component',
				onNextClick: () => {
					updateProgress(7)

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		}
	]}
/>

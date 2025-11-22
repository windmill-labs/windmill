<script lang="ts">
	import { insertNewGridItem, appComponentFromType } from '$lib/components/apps/editor/appUtils'
	import type { AppComponent, TypedComponent } from '$lib/components/apps/editor/component'
	import type { AppViewerContext, AppEditorContext } from '$lib/components/apps/types'
	import { push } from '$lib/history.svelte'
	import { getContext } from 'svelte'
	import Tutorial from '../Tutorial.svelte'
	import {
		clickButtonBySelector,
		clickFirstButtonBySelector,
		connectInlineRunnableInputToComponentOutput,
		isAppTainted,
		updateInlineRunnableCode
	} from '../utils'
	import { updateProgress } from '$lib/tutorialUtils'
	import { type DriveStep } from 'driver.js'
	import { wait } from '$lib/utils'

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
	tainted={isAppTainted($app)}
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'App editor tutorial',
					description:
						'This tutorial will show you how to use the App editor, add components, background scripts and connect them.',
					onNextClick: () => {
						addComponent('textinputcomponent')

						setTimeout(() => {
							clickButtonBySelector('#app-editor-component-library-tab')
						})

						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},

			{
				element: '#app-editor-component-list',
				popover: {
					title: 'Components panel',
					description:
						'This is the components panel. Here you can add components to your app. Components are the building blocks of your app. You can add as many components as you want.'
				}
			},
			{
				element: '#displaycomponent',
				popover: {
					title: 'Adding a component',
					description:
						'Click on a component to add it to your app. Here we will add a display component.',
					onNextClick: () => {
						if (!$selectedComponent?.includes('e')) {
							addComponent('displaycomponent')
						}

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
				element: '#component-input',
				popover: {
					title: 'Component input',
					description:
						'There are several ways to set the input of a component. It can be static, the result of a JS expression, connected to the output of another component, or the result of an inline runnable. Here we will create an inline runnable that will convert the text to uppercase.',
					onNextClick: () => {
						clickFirstButtonBySelector('#component-input')
						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},

			{
				element: '#data-source-compute',
				popover: {
					title: 'Compute',
					description: 'Click on the compute button to create a new inline runnable.',
					onNextClick: () => {
						clickButtonBySelector('#data-source-compute')
						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},

			{
				element: '#app-editor-create-inline-script',
				popover: {
					title: 'Create an inline script',
					description: "Let's create an inline script.",
					onNextClick: () => {
						clickButtonBySelector('#app-editor-create-inline-script')
						setTimeout(() => driver.moveNext())
					}
				}
			},

			{
				element: '#app-editor-empty-runnable',
				popover: {
					title: 'Choose a language',
					description:
						'You can choose the language of your runnable. There are two type of runnables: frontend and backend.'
				}
			},

			{
				element: '#app-editor-backend-runnables',
				popover: {
					title: 'Backend runnables',
					description:
						'Backend runnables are scripts that are executed on the server. They can be used to perform tasks that are not possible to be performed on the client. For example, you can use backend runnables to send emails, perform database operations, etc.'
				}
			},
			{
				element: '#app-editor-frontend-runnables',
				popover: {
					title: 'Frontend runnables',
					description:
						'Frontend scripts are executed in the browser and can manipulate the app context directly. You can also interact with components using component controls.'
				}
			},
			{
				element: '#create-deno-script',
				onHighlighted: () => {
					document.querySelector('#schema-plug-x')?.parentElement?.classList.remove('opacity-0')
				},
				popover: {
					title: 'Create a deno script',
					description:
						"Let's create a simple deno script. For the sake of this tutorial, we will create a script that converts the text to uppercase.",
					onNextClick: async () => {
						clickButtonBySelector('#create-deno-script')
						await wait(50)
						if ($selectedComponent?.[0]) {
							updateInlineRunnableCode(
								$app,
								$selectedComponent[0],
								'export function main(x: string) {\n  return x?.toLocaleUpperCase();\n}'
							)
						}

						driver.moveNext()
					}
				}
			},
			{
				element: '#schema-plug-x',
				onHighlighted: () => {
					document.querySelector('#schema-plug-x')?.parentElement?.classList.remove('opacity-0')
				},
				popover: {
					title: 'Connect the function input',
					description:
						"The function we created has an string input 'x'. We can connect the output of the text component to it.",
					onNextClick: () => {
						clickButtonBySelector('#schema-plug-x')
						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},
			{
				element: '#connect-output-a',
				popover: {
					title: 'Select the output',
					description: 'Open the output selector of the text input component.',
					onNextClick: () => {
						clickButtonBySelector('#connect-output-a')
						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},
			{
				element: '.component-output-viewer-a li *:has(> button[title="result"])',
				popover: {
					title: 'Select the output',
					description: "Let's select the result of the text input component.",
					onNextClick: async () => {
						clickButtonBySelector('.component-output-viewer-a li button[title="result"]')
						await wait(100)
						driver.moveNext()
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
						connectInlineRunnableInputToComponentOutput($app, 'e', 'x', 'd', 'result', 'integer')

						$app = $app

						updateProgress(7)

						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			}
		]

		return steps
	}}
/>

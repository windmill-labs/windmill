<script lang="ts">
	import { insertNewGridItem, appComponentFromType } from '$lib/components/apps/editor/appUtils'
	import type { AppComponent, TypedComponent } from '$lib/components/apps/editor/component'
	import type { AppViewerContext, AppEditorContext } from '$lib/components/apps/types'
	import { push } from '$lib/history'
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

	export let name: string
	export let index: number

	let tutorial: Tutorial | undefined = undefined

	const { app, selectedComponent, focusedGrid, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')
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
		const steps = [
			{
				popover: {
					title: 'Welcome to the App editor tutorial !',
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
						if (!$selectedComponent?.includes('b')) {
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
						'They are several ways to set the input of a component. It can be static, the result of a JS expression, connected to the output of another component, or the result of a inline runnable. Here we will create an inline runnable that will convert the text to uppercase.',
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
						clickFirstButtonBySelector('#data-source-compute')
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
						'You can choose the language of your runnable. They are two type of runnables: frontend and backend.'
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
				popover: {
					title: 'Create a deno script',
					description:
						"Let's create a simple deno script. For the sake of this tutorial, we will create a script that converts the text to uppercase.",
					onNextClick: () => {
						clickButtonBySelector('#create-deno-script')
						setTimeout(() => {
							if ($selectedComponent?.[0]) {
								updateInlineRunnableCode(
									$app,
									$selectedComponent[0],
									`export async function main(x: string) {
  return x?.toLocaleUpperCase();
}
`
								)
							}

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
				element: '#connect-output-a',
				popover: {
					title: 'Select the output',
					description: ' ',
					onNextClick: () => {
						$connectingInput.opened = false
						$connectingInput.input = undefined

						setTimeout(() => {
							driver.moveNext()
						})
					},
					onPopoverRender: (popover, opts) => {
						const wrapper = document.createElement('div')
						wrapper.classList.add('flex', 'flex-col', 'gap-2', 'w-full', 'items-start')

						const p1 = document.createElement('p')
						p1.innerText =
							'You can now select the output in the output menu. Click on the little red button to open the menu.'

						const id = document.createElement('div')
						id.innerHTML = `<button class="bg-red-500/70 border border-red-600 px-1 py-0.5" title="Outputs" aria-label="Open output"><svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide-icon lucide lucide-plug-2 "><path d="M9 2v6"></path><path d="M15 2v6"></path><path d="M12 17v5"></path><path d="M5 8h14"></path><path d="M6 11V8h12v3a6 6 0 1 1-12 0v0Z"></path></svg></button>`

						const p2 = document.createElement('p')
						p2.innerText =
							'Once opened, you can select the output you want to connect to. Here we will connect the result output of the text component to the input "x" of the inline runnable.'

						const objectViewer = document.createElement('div')
						objectViewer.innerHTML = `<div class="rounded-lg shadow-md border p-4 bg-surface"><span class="s-UNyBDXJ1E286">  <ul class="w-full pl-2 border-none s-UNyBDXJ1E286"><li class="s-UNyBDXJ1E286"><button class="whitespace-nowrap s-UNyBDXJ1E286"><span class="key border  font-semibold rounded px-1 hover:bg-surface-hover text-2xs text-secondary s-UNyBDXJ1E286">result</span> :</button> <button class="val  rounded px-1 hover:bg-blue-100 string s-UNyBDXJ1E286"><span title="" class="text-2xs s-UNyBDXJ1E286">""</span></button></li> </ul> </span> <span class="border border-blue-600 rounded px-1 cursor-pointer hover:bg-gray-200 s-UNyBDXJ1E286 hidden">{...}</span> </div>`

						wrapper.appendChild(p1)
						wrapper.appendChild(id)
						wrapper.appendChild(p2)
						wrapper.appendChild(objectViewer)

						popover.description.appendChild(wrapper)

						tutorial?.renderControls(opts)
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
						connectInlineRunnableInputToComponentOutput($app, 'b', 'x', 'a', 'result')

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

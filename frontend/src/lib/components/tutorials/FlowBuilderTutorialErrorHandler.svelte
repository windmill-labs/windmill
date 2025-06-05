<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import Tutorial from './Tutorial.svelte'
	import { clickButtonBySelector, triggerPointerDown } from './utils'
	import { updateProgress } from '$lib/tutorialUtils'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={4}
	name="error-handler"
	tainted={false}
	on:error
	on:skipAll
	getSteps={(driver) => [
		{
			popover: {
				title: 'Error handler tutorial',
				description: 'Learn how to recover from an error. You can use arrow keys to navigate.',
				onNextClick: () => {
					flowStore.value.modules = [
						{
							id: 'a',
							value: {
								type: 'rawscript',
								content:
									'// import * as wmill from "npm:windmill-client@1"\n\nexport async function main(x: string) {\n  throw new Error("Fake error")\n}\n',
								language: 'deno',
								input_transforms: {
									x: {
										type: 'static',
										value: ''
									}
								}
							}
						}
					]
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#flow-editor-error-handler',
			popover: {
				title: 'Error handler',
				description:
					'You can add an error handler to your flow. It will be executed if any of the steps in the flow fails.',

				onNextClick: () => {
					triggerPointerDown('#flow-editor-error-handler button')
					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			popover: {
				title: 'Steps kind',
				description: "Choose the kind of step you want to add. Let's start with a simple action"
			},
			element: '#flow-editor-insert-module'
		},
		{
			element: '#flow-editor-flow-providers',
			popover: {
				title: 'Action configuration',
				description: 'An action can be inlined, imported from your workspace or the Hub.'
			}
		},
		{
			element: '#flow-editor-flow-atoms',
			popover: {
				title: 'Supported languages',
				description: 'Windmill support the following languages/runtimes.'
			}
		},
		{
			element: '#flow-editor-new-bun',
			popover: {
				title: 'Typescript',
				description: "Let's create a Typescript error handler for your flow",
				onNextClick: () => {
					clickButtonBySelector('#flow-editor-new-bun')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#flow-editor-test-flow',
			popover: {
				title: 'Test your flow',
				description: 'We can now test our flow',
				onNextClick: () => {
					clickButtonBySelector('#flow-editor-test-flow')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},

		{
			element: '#flow-editor-test-flow-drawer',
			popover: {
				title: 'Test your flow',
				description:
					'Finally we can test our flow, and view how the error handler is executed when a step fails',
				onNextClick: () => {
					clickButtonBySelector('#flow-editor-test-flow-drawer')

					setTimeout(() => {
						driver.moveNext()

						updateProgress(4)
					})
				}
			}
		}
	]}
/>

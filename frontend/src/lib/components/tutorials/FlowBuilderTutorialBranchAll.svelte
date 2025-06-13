<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { triggerPointerDown } from './utils'
	import Tutorial from './Tutorial.svelte'
	import { updateProgress } from '$lib/tutorialUtils'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial(indexToInsertAt?: number | undefined) {
		tutorial?.runTutorial({ indexToInsertAt })
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={3}
	name="branchall"
	on:error
	on:skipAll
	getSteps={(driver, options) => {
		const index = options?.indexToInsertAt ?? flowStore.val.value.modules.length

		const steps = [
			{
				popover: {
					title: 'Branch all tutorial',
					description:
						'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate'
				}
			},
			{
				element: `#flow-editor-add-step-${index}`,
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						triggerPointerDown(`#flow-editor-add-step-${index}`)

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
				popover: {
					title: 'Insert Branch all',
					description: "Let's pick branch all",
					onNextClick: () => {
						triggerPointerDown('#flow-editor-flow-kind-branch-to-all')

						setTimeout(() => {
							driver.moveNext()
						})
					}
				},
				element: '#flow-editor-flow-kind-branch-to-all'
			},
			{
				element: '#flow-editor-branch-all-wrapper',
				popover: {
					title: 'Branches',
					description:
						'Here you can add a summary to a branch, or configure the branches to run in parallel',
					onNextClick: () => {
						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},
			{
				popover: {
					title: 'Add steps',
					description: 'You can now add step to one of the branches',
					onNextClick: () => {
						setTimeout(() => {
							driver.moveNext()

							updateProgress(3)
						})
					}
				}
			}
		]

		return steps
	}}
/>

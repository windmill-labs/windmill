<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { triggerAddFlowStep, selectFlowStepKind } from './utils'
	import Tutorial from './Tutorial.svelte'
	import { updateProgress } from '$lib/tutorialUtils'
	import { nextId } from '../flows/flowModuleNextId'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

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
		const id = nextId($flowStateStore, $flowStore)
		const index = options?.indexToInsertAt ?? $flowStore.value.modules.length
		const isFirst = id === 'a'

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
						triggerAddFlowStep(index)

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
						selectFlowStepKind(isFirst ? 6 : 5)

						setTimeout(() => {
							driver.moveNext()
						})
					}
				},
				element: `#flow-editor-insert-module > div > button:nth-child(${isFirst ? 6 : 5})`
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

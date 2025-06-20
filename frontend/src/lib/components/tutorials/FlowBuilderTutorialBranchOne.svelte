<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { clickButtonBySelector, updateFlowModuleById, triggerPointerDown } from './utils'
	import Tutorial from './Tutorial.svelte'
	import { updateProgress } from '$lib/tutorialUtils'
	import { nextId } from '../flows/flowModuleNextId'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial(indexToInsertAt?: number | undefined) {
		tutorial?.runTutorial({ indexToInsertAt })
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={2}
	name="branchone"
	on:error
	on:skipAll
	getSteps={(driver, options) => {
		const id = nextId($flowStateStore, flowStore.val)
		const index = options?.indexToInsertAt ?? flowStore.val.value.modules.length

		const steps = [
			{
				popover: {
					title: 'Branch one tutorial',
					description:
						'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate'
				}
			},
			{
				element: `#flow-editor-add-step-${index}`,
				popover: {
					title: 'Branch one',
					description: 'Windmill supports branches, let us add one',
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
					description: 'Choose the kind of step you want to add.'
				},
				element: '#flow-editor-insert-module'
			},
			{
				popover: {
					title: 'Insert Branch one',
					description: "Let's pick branch one",
					onNextClick: () => {
						triggerPointerDown('#flow-editor-flow-kind-branch-to-one')

						setTimeout(() => {
							driver.moveNext()
						})
					}
				},
				element: '#flow-editor-flow-kind-branch-to-one'
			},
			{
				element: '#flow-editor-edit-predicate',
				popover: {
					title: 'Edit predicate',
					description: 'Click here to edit the predicate of your branch',
					onNextClick: () => {
						clickButtonBySelector('#flow-editor-edit-predicate')

						updateFlowModuleById(flowStore.val, id, (module) => {
							if (module.value.type === 'branchone') {
								module.value.branches[0].expr = "result.a === 'foo'"
							}
						})

						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},
			{
				element: '#flow-editor-branch-one-wrapper',
				popover: {
					title: 'Predicate saved',
					description: 'You can now see the predicate of your branch',
					onNextClick: () => {
						dispatch('reload')

						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},
			{
				popover: {
					title: 'Add steps',
					description: 'You can now add a step to one of the branches',
					onNextClick: () => {
						setTimeout(() => {
							updateProgress(2)
							driver.moveNext()
						})
					}
				}
			}
		]
		return steps
	}}
/>

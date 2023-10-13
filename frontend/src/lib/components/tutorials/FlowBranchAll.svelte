<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import {
		clickButtonBySelector,
		triggerAddFlowStep,
		selectFlowStepKind,
		updateFlowModuleById
	} from './utils'
	import { updateProgress } from '$lib/tutorialUtils'
	import { RawScript } from '$lib/gen'
	import Tutorial from './Tutorial.svelte'
	import { nextId } from '../flows/flowStateUtils'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial(indexToInsertAt?: number | undefined) {
		tutorial?.runTutorial(indexToInsertAt)
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={3}
	name="branchall"
	on:error
	on:skipAll
	getSteps={(driver, indexToInsertAt) => {
		const id = nextId($flowStateStore, $flowStore)
		const index = indexToInsertAt ?? $flowStore.value.modules.length
		const isFirst = id === 'a'

		const steps = [
			{
				popover: {
					title: 'Welcome to the Windmil Flow editor',
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
				element: '#add-branch-button',
				popover: {
					title: 'Add branch',
					description: 'Click here to add a branch',
					onNextClick: () => {
						clickButtonBySelector('#add-branch-button')

						setTimeout(() => {
							driver.moveNext()
						})
					}
				}
			},

			{
				element: '#flow-editor-step-input',
				popover: {
					title: 'Branch all',
					description: 'All branches will be executed, and the result will be gathered in an array',

					onNextClick: () => {
						updateFlowModuleById($flowStore, id, (module) => {
							module.value = {
								type: 'branchall',
								branches: [
									{
										modules: [
											{
												id: nextId($flowStateStore, $flowStore),
												value: {
													type: 'rawscript',
													content:
														'// import * as wmill from "npm:windmill-client@1"\n\nexport async function main(x: string) {\n  return "Hello"\n}\n',
													language: RawScript.language.DENO,
													input_transforms: {
														x: {
															type: 'static'
														}
													}
												}
											}
										],
										skip_failure: false
									},
									{
										summary: '',
										modules: [
											{
												id: 'last-branch',
												value: {
													type: 'rawscript',
													content:
														'package inner\n\nfunc main() (interface{}, error) {\n\treturn "World", nil\n}\n',
													language: RawScript.language.GO,
													input_transforms: {}
												}
											}
										],
										skip_failure: false
									}
								],
								parallel: false
							}
						})

						$flowStore = $flowStore
						dispatch('reload')

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
					description: 'Finally we can test our flow, and view the results!',
					onNextClick: () => {
						clickButtonBySelector('#flow-editor-test-flow-drawer')

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

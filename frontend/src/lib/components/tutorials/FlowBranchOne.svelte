<script lang="ts">
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import TutorialItem from './TutorialItem.svelte'
	import {
		clickButtonBySelector,
		setInputBySelector,
		triggerAddFlowStep,
		selectFlowStepKind
	} from './utils'
	import { updateProgress } from '$lib/tutorialUtils'
	import { RawScript } from '$lib/gen'

	export let shouldRenderButton: boolean = true

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	export function runTutorial() {
		const branchOneTutorial = driver({
			showProgress: true,
			allowClose: true,
			onPopoverRender: (popover, { config, state }) => {
				if (state.activeIndex == 0) {
					const skipThisButton = document.createElement('button')
					skipThisButton.innerText = 'Skip this tutorials'
					skipThisButton.addEventListener('click', () => {
						updateProgress(2)

						branchOneTutorial.destroy()
					})
					skipThisButton.setAttribute(
						'class',
						'border px-2 py-1 !text-xs font-normal rounded-md hover:bg-surface-hover transition-all flex items-center'
					)

					const skipAllButton = document.createElement('button')
					skipAllButton.innerText = 'Skip all tutorials'
					skipAllButton.addEventListener('click', () => {
						dispatch('skipAll')
						branchOneTutorial.destroy()
					})

					skipAllButton.setAttribute(
						'class',
						'border px-2 py-1 !text-xs font-normal rounded-md hover:bg-surface-hover transition-all flex items-center'
					)

					const popoverDescription = document.querySelector('#driver-popover-description')

					const div = document.createElement('div')

					div.setAttribute('class', 'flex flex-row gap-2 justify-between w-full pt-2')

					if (popoverDescription) {
						div.appendChild(skipAllButton)
						div.appendChild(skipThisButton)

						popoverDescription.appendChild(div)
					}
				}
			},
			steps: [
				{
					popover: {
						title: 'Welcome to the Windmil Flow editor',
						description:
							'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate'
					}
				},

				{
					popover: {
						title: 'Flows inputs',
						description: 'Flows have inputs that can be used in the flow',
						onNextClick: () => {
							clickButtonBySelector('#flow-editor-virtual-Input')

							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					},
					element: '#svelvet-Input'
				},
				{
					element: '#flow-editor-add-property',
					popover: {
						title: 'Add a property',
						description: 'Click here to add a property to your schema',
						onNextClick: () => {
							clickButtonBySelector('#flow-editor-add-property')
							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					}
				},
				{
					element: '#schema-modal-name',
					popover: {
						title: 'Name your property',
						description: 'Give a name to your property. Here we will call it condition',
						onNextClick: () => {
							setInputBySelector('#schema-modal-name', 'condition')
							branchOneTutorial.moveNext()
						}
					}
				},
				{
					element: '#schema-modal-type-boolean',
					popover: {
						title: 'Property type',
						description: 'Choose the type of your property. Here we will choose boolean',
						onNextClick: () => {
							clickButtonBySelector('#schema-modal-type-boolean')
							branchOneTutorial.moveNext()
						}
					}
				},
				{
					element: '#schema-modal-save',
					popover: {
						title: 'Save your property',
						description: 'Click here to save your property',
						onNextClick: () => {
							clickButtonBySelector('#schema-modal-save')

							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '#flow-editor-add-step-0',
					popover: {
						title: 'Branch one',
						description: 'Windmill supports branches, let us add one',
						onNextClick: () => {
							triggerAddFlowStep(0)

							setTimeout(() => {
								branchOneTutorial.moveNext()
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
							selectFlowStepKind(5)

							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					},
					element: '#flow-editor-insert-module > div > button:nth-child(5)'
				},

				{
					element: '#add-branch-button',
					popover: {
						title: 'Add branch',
						description: 'Click here to add a branch',
						onNextClick: () => {
							clickButtonBySelector('#add-branch-button')

							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '#flow-editor-edit-predicate',
					popover: {
						title: 'Edit predicate',
						description: 'Click here to edit the predicate of your branch',
						onNextClick: () => {
							clickButtonBySelector('#flow-editor-edit-predicate')

							setTimeout(() => {
								branchOneTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '.key',
					popover: {
						title: 'Connect',
						description: 'As we did before, we can connect to the iterator of the loop',
						onNextClick: () => {
							if ($flowStore.value.modules[0].value.type === 'branchone') {
								$flowStore.value.modules[0].value.branches[0].expr = 'flow_input.condition'
							}

							$flowStore = $flowStore
							dispatch('reload')

							tick().then(() => {
								branchOneTutorial.moveNext()
							})
						}
					}
				},

				//flow-editor-add-step-

				{
					popover: {
						title: 'Branche modules',
						description:
							'We can now add modules to each branch, one in typescript and one in python',
						onNextClick: () => {
							if ($flowStore.value.modules[0].value.type === 'branchone') {
								$flowStore.value.modules[0].value = {
									type: 'branchone',
									branches: [
										{
											summary: '',
											expr: 'flow_input.condition',
											modules: [
												{
													id: 'c',
													value: {
														type: 'rawscript',
														content:
															'export async function main() {\n  return "Entered the condition!";\n}\n',
														language: RawScript.language.DENO,
														input_transforms: {}
													}
												}
											]
										}
									],
									default: [
										{
											id: 'b',
											value: {
												type: 'rawscript',
												content: '# import wmill\n\n\ndef main():\n    return "Default Branch"',
												language: RawScript.language.PYTHON3,
												input_transforms: {}
											}
										}
									]
								}
							}
							$flowStore = $flowStore
							dispatch('reload')

							tick().then(() => {
								branchOneTutorial.moveNext()
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
								branchOneTutorial.moveNext()
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
								branchOneTutorial.moveNext()
								updateProgress(2)
							})
						}
					}
				}
			]
		})
		branchOneTutorial.drive()
	}
</script>

{#if shouldRenderButton}
	<TutorialItem
		on:click={() => runTutorial()}
		label="Branch one tutorial"
		index={2}
		id="flow-builder-tutorial-branchone"
	/>
{/if}

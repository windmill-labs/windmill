<script lang="ts">
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { updateProgress } from '$lib/tutorialUtils'
	import TutorialItem from './TutorialItem.svelte'
	import {
		clickButtonBySelector,
		selectFlowStepKind,
		setInputBySelector,
		triggerAddFlowStep
	} from './utils'
	import { tutorialsToDo } from '$lib/stores'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	const queue: string[] = []

	const simpleFlowTutorial = driver({
		showProgress: true,
		allowClose: true,
		onPopoverRender: (popover, { config, state }) => {
			if (state.activeIndex == 0) {
				const skipThisButton = document.createElement('button')
				skipThisButton.innerText = 'Skip this tutorials'
				skipThisButton.addEventListener('click', () => {
					updateProgress(0)
					simpleFlowTutorial.destroy()
				})
				skipThisButton.setAttribute(
					'class',
					'border px-2 py-1 !text-xs font-normal rounded-md hover:bg-surface-hover transition-all flex items-center'
				)

				const skipAllButton = document.createElement('button')
				skipAllButton.innerText = 'Skip all tutorials'
				skipAllButton.addEventListener('click', () => {
					dispatch('skipAll')
					simpleFlowTutorial.destroy()
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
					description: 'Learn how to build powerful flows in a few steps'
				}
			},
			{
				popover: {
					title: 'Flows inputs',
					description: 'Flows have inputs that can be used in the flow',
					onNextClick: () => {
						clickButtonBySelector('#flow-editor-virtual-Input')
						setTimeout(() => {
							simpleFlowTutorial.moveNext()
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
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},
			{
				element: '#schema-modal-name',
				popover: {
					title: 'Name your property',
					description: 'Give a name to your property. Here we will call it firstname',
					onNextClick: () => {
						setInputBySelector('#schema-modal-name', 'firstname')

						simpleFlowTutorial.moveNext()
					}
				}
			},
			{
				element: '#schema-modal-save',
				popover: {
					title: 'Save your property',
					description: 'Click here to save your property',
					onNextClick: () => {
						queue.push(JSON.stringify($flowStore))

						clickButtonBySelector('#schema-modal-save')

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},
			{
				element: '#flow-editor-add-step-0',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						triggerAddFlowStep(0)

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
						})
					},

					onPrevClick: () => {
						simpleFlowTutorial.moveTo(1)
						debugger
						$flowStore = JSON.parse(queue.pop() || '{}')
						dispatch('reload')
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
					title: 'Pick an action',
					description: 'Let’s pick an action to add to your flow',
					onNextClick: () => {
						selectFlowStepKind(1)

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
						})
					}
				},
				element: '#flow-editor-insert-module > div > button:nth-child(1)'
			},
			{
				element: '#flow-editor-flow-inputs',
				popover: {
					title: 'Action configuration',
					description: 'An action can be inlined, imported from your workspace or the Hub.'
				}
			},
			{
				element: '#flow-editor-action-script',
				popover: {
					title: 'Supported languages',
					description: 'Windmill support the following languages/runtimes.'
				}
			},
			{
				element: '#flow-editor-action-script > button:nth-child(1)',
				popover: {
					title: 'Typescript',
					description: "Let's pick an action to add to your flow",
					onNextClick: () => {
						clickButtonBySelector('#flow-editor-action-script > button > div > button:nth-child(1)')

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},

			{
				element: '#flow-editor-editor',
				popover: {
					title: 'Action editor',
					description: 'Windmill provides a full code editor to write your actions'
				}
			},

			{
				element: '#flow-editor-step-input',
				popover: {
					title: 'Autogenerated schema',
					description: 'The schema and the UI is autogenerated from your code'
				}
			},

			{
				element: '#flow-editor-plug',
				popover: {
					title: 'Connect',
					description:
						'You can provide static values or connect to other nodes result. Here we will connect to the firstname input',
					onNextClick: () => {
						clickButtonBySelector('#flow-editor-plug')

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},
			{
				element: '.key',
				popover: {
					title: 'Connection mode',
					description: 'Once you pressed the connect button, you can choose what to connect to.',
					onNextClick: () => {
						if ($flowStore.value.modules[0].value.type === 'rawscript') {
							$flowStore.value.modules[0].value.input_transforms = {
								x: {
									type: 'javascript',
									expr: 'flow_input.firstname'
								}
							}
						}

						$flowStore = $flowStore
						dispatch('reload')

						tick().then(() => {
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},

			{
				element: '#flow-editor-step-input',
				popover: {
					title: 'Input connected!',
					description: 'The input is now connected to the firstname input'
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
							simpleFlowTutorial.moveNext()
						})
					}
				}
			},

			{
				element: '#flow-preview-content',
				popover: {
					title: 'Flow input',
					description: 'Let’s provide an input to our flow',
					onNextClick: () => {
						setInputBySelector('textarea.w-full', 'Hello World!')

						setTimeout(() => {
							simpleFlowTutorial.moveNext()
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
							simpleFlowTutorial.moveNext()

							updateProgress(0)
						})
					}
				}
			}
		]
	})

	$: $tutorialsToDo.includes(0) && simpleFlowTutorial.drive()
</script>

<TutorialItem
	on:click={() => simpleFlowTutorial.drive()}
	label="Simple flow tutorial"
	index={0}
	id="flow-builder-tutorial-action"
/>

<script lang="ts">
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import TutorialItem from './TutorialItem.svelte'
	import { emptyFlowModuleState } from '../flows/utils'
	import {
		clickButtonBySelector,
		setInputBySelector,
		triggerAddFlowStep,
		selectFlowStepKind,
		selectOptionsBySelector
	} from './utils'
	import { updateProgress } from '$lib/tutorialUtils'

	const { flowStore, selectedId, flowStateStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()

	function runTutorial() {
		const forloopTutorial = driver({
			showProgress: true,
			allowClose: true,
			steps: [
				{
					popover: {
						title: 'Welcome in the Windmil Flow editor',
						description: 'Learn how to build our first for loop to iterate on'
					}
				},

				{
					popover: {
						title: 'Flows inputs',
						description: 'Flows have inputs that can be used in the flow',
						onNextClick: () => {
							clickButtonBySelector('#flow-editor-virtual-Input')

							setTimeout(() => {
								forloopTutorial.moveNext()
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
								forloopTutorial.moveNext()
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
							setInputBySelector('#schema-modal-name', 'array')
							forloopTutorial.moveNext()
						}
					}
				},
				{
					element: '#schema-modal-type-array',
					popover: {
						title: 'Property type',
						description: 'Choose the type of your property. Here we will choose array',
						onNextClick: () => {
							clickButtonBySelector('#schema-modal-type-array')
							forloopTutorial.moveNext()
						}
					}
				},
				{
					element: '#array-type-narrowing',
					popover: {
						title: 'Array type narrowing',
						description: 'You can narrow the type of your array. Here we will choose numbers',
						onNextClick: () => {
							selectOptionsBySelector('#array-type-narrowing', 'number')
							forloopTutorial.moveNext()
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
								forloopTutorial.moveNext()
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
								forloopTutorial.moveNext()
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
						title: 'Insert loop',
						description: "Let's pick forloop",
						onNextClick: () => {
							selectFlowStepKind(4)
							setTimeout(() => {
								forloopTutorial.moveNext()
							})
						}
					},
					element: '#flow-editor-insert-module > div > button:nth-child(4)'
				},

				{
					element: '#flow-editor-iterator-expression',
					popover: {
						title: 'Iterator expression',
						description:
							'The iterator expression is a javascript expression that respresents the array to iterate on. Here we will iterate on the firstname input letter by letter',
						onNextClick: () => {
							if ($flowStore.value.modules[0].value.type === 'forloopflow') {
								if ($flowStore.value.modules[0].value.iterator.type === 'javascript') {
									$flowStore.value.modules[0].value.iterator.expr = 'flow_input.array'
								}
							}

							$flowStore = $flowStore

							dispatch('reload')

							tick().then(() => {
								forloopTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '#flow-editor-iterator-expression',
					popover: {
						title: 'Iterator expression',
						description:
							'We can refer to the result of previous steps using the results object: results.a',
						onNextClick: () => {
							if ($flowStore.value.modules[0].value.type === 'forloopflow') {
								$flowStore.value.modules[0].value.modules = [
									{
										id: 'b',
										value: {
											type: 'rawscript',
											content: 'def main(x: int):\n    return x*2',
											// @ts-ignore
											language: 'python3',
											input_transforms: {
												x: {
													type: 'javascript',
													// @ts-ignore
													value: '',
													expr: ''
												}
											}
										}
									}
								]
							}

							$flowStateStore['b'] = emptyFlowModuleState()

							$flowStateStore['b'].schema.properties = {
								x: {
									type: 'number',
									description: '',
									default: null
								}
							}

							$flowStore = $flowStore

							dispatch('reload')

							tick().then(() => {
								forloopTutorial.moveNext()
							})
						}
					}
				},
				{
					element: '#svelvet-c',
					popover: {
						title: 'Step of the loop',
						description: 'We added an action to the loop. Let’s configure it',
						onNextClick: () => {
							$selectedId = 'b'
							$flowStore = $flowStore

							dispatch('reload')
							tick().then(() => {
								forloopTutorial.moveNext()
							})
						}
					}
				},
				{
					element: '#flow-editor-editor',
					popover: {
						title: 'Python step',
						description:
							'We can write python code in the editor. In this example we will capitalize the letter'
					}
				},

				{
					element: '#flow-editor-step-input',
					popover: {
						title: 'flow editor',
						description: 'Description'
					}
				},

				{
					element: '#flow-editor-plug',
					popover: {
						title: 'Input configuration',
						description:
							'UI is autogenerated from your code. Let’s connect the input to the letter input',
						onNextClick: () => {
							clickButtonBySelector('#flow-editor-plug')

							setTimeout(() => {
								forloopTutorial.moveNext()
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
							if (
								$flowStore.value.modules[0].value.type === 'forloopflow' &&
								$flowStore.value.modules[0].value.modules[0].value.type === 'rawscript'
							) {
								$flowStore.value.modules[0].value.modules[0].value.input_transforms = {
									x: {
										type: 'javascript',
										expr: 'flow_input.iter.value'
									}
								}
							}

							$flowStore = $flowStore
							dispatch('reload')

							tick().then(() => {
								forloopTutorial.moveNext()
							})
						}
					}
				},
				{
					element: '#flow-editor-step-input',
					popover: {
						title: 'Iterator',
						description:
							'Loops expose an iterator object that contains the current value of the loop and the index'
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
								forloopTutorial.moveNext()
							})
						}
					}
				},

				{
					element: 'arg-input-add-item',
					popover: {
						title: 'Flow input',
						description: 'Let’s add an item to our array',
						onNextClick: () => {
							clickButtonBySelector('#arg-input-add-item')

							setTimeout(() => {
								forloopTutorial.moveNext()
							})
						}
					}
				},
				{
					element: 'arg-input-add-item',
					popover: {
						title: 'Flow input',
						description: 'We can set the value of the item',
						onNextClick: () => {
							setInputBySelector('#arg-input-number-array', '25')

							setTimeout(() => {
								forloopTutorial.moveNext()
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
								forloopTutorial.moveNext()
								updateProgress(1)
							})
						}
					}
				}
			]
		})
		forloopTutorial.drive()
	}
</script>

<TutorialItem on:click={() => runTutorial()} label="For loops tutorial" index={1} />

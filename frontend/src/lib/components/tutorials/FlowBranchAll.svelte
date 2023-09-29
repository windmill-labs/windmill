<script lang="ts">
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let renderCount = 1

	function runTutorial() {
		const branchAllTutorial = driver({
			showProgress: true,
			allowClose: true,
			steps: [
				{
					popover: {
						title: 'Welcome in the Windmil Flow editor',
						description: 'Learn how to build powerfull flows in a few steps'
					}
				},

				{
					popover: {
						title: 'Flows inputs',
						description: 'Flows have inputs that can be used in the flow',
						onNextClick: () => {
							const button = document.querySelector(
								'#flow-editor-virtual-Input'
							) as HTMLButtonElement
							if (button) {
								button.click()
							}
							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							const button = document.querySelector(
								'#flow-editor-add-property'
							) as HTMLButtonElement
							if (button) {
								button.click()
							}
							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							const input = document.querySelector('#schema-modal-name') as HTMLInputElement

							if (input) {
								input.value = 'condition'
								input.dispatchEvent(new Event('input', { bubbles: true }))
							}
							branchAllTutorial.moveNext()
						}
					}
				},
				{
					element: '#schema-modal-type-boolean',
					popover: {
						title: 'Property type',
						description: 'Choose the type of your property. Here we will choose boolean',
						onNextClick: () => {
							const button = document.querySelector(
								'#schema-modal-type-boolean'
							) as HTMLButtonElement

							if (button) {
								button.click()
							}
							branchAllTutorial.moveNext()
						}
					}
				},
				{
					element: '#schema-modal-save',
					popover: {
						title: 'Save your property',
						description: 'Click here to save your property',
						onNextClick: () => {
							const button = document.querySelector('#schema-modal-save') as HTMLButtonElement

							if (button) {
								button.click()
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '#flow-editor-add-step-0',
					popover: {
						title: 'Loops',
						description: 'Windmill supports loops. Let’s add a loop to your flow',
						onNextClick: () => {
							const button = document.querySelector('#flow-editor-add-step-0') as HTMLButtonElement
							if (button) {
								button.parentElement?.dispatchEvent(
									new PointerEvent('pointerdown', { bubbles: true })
								)
							}
							setTimeout(() => {
								branchAllTutorial.moveNext()
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
						title: 'Insert loop',
						description: "Let's pick forloop",
						onNextClick: () => {
							const button = document.querySelector(
								'#flow-editor-insert-module > div > button:nth-child(6)'
							) as HTMLButtonElement

							if (button) {
								button?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
							})
						}
					},
					element: '#flow-editor-insert-module > div > button:nth-child(6)'
				},

				{
					element: '#add-branch-button',
					popover: {
						title: 'Add branch',
						description: 'Click here to add a branch to your loop',
						onNextClick: () => {
							const button = document.querySelector('#add-branch-button') as HTMLButtonElement

							if (button) {
								button.click()
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							const button = document.querySelector(
								'#flow-editor-edit-predicate'
							) as HTMLButtonElement

							if (button) {
								button.click()
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							renderCount += 1

							tick().then(() => {
								branchAllTutorial.moveNext()
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
							const button = document.querySelector('#flow-editor-test-flow') as HTMLButtonElement

							if (button) {
								button?.click()
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
							})
						}
					}
				},

				{
					element: 'textarea.w-full',
					popover: {
						title: 'Flow input',
						description: 'Let’s provide an input to our flow',
						onNextClick: () => {
							const textarea = document.querySelector('textarea.w-full') as HTMLTextAreaElement

							if (textarea) {
								textarea.value = 'Hello World!'
								textarea.dispatchEvent(new Event('input', { bubbles: true }))
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							const button = document.querySelector(
								'#flow-editor-test-flow-drawer'
							) as HTMLButtonElement

							if (button) {
								button?.click()
							}

							setTimeout(() => {
								branchAllTutorial.moveNext()
							})
						}
					}
				}
			]
		})
		branchAllTutorial.drive()
	}
</script>

<MenuItem
	on:click={() => {
		runTutorial()
	}}
>
	<div
		class={classNames(
			'text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 hover:text-primary-inverse !text-xs font-semibold'
		)}
	>
		Branch all tutorial
	</div>
</MenuItem>

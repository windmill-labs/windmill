<script lang="ts">
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { clickButtonBySelector, triggerAddFlowStep, selectFlowStepKind } from './utils'
	import { updateProgress } from '$lib/tutorialUtils'
	import { RawScript } from '$lib/gen'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()

	export function runTutorial() {
		if (
			$flowStore.value.modules.length > 0 ||
			Object.keys($flowStore?.schema?.properties).length > 0
		) {
			dispatch('error', { detail: 'branchall' })
			return
		}

		const branchAllTutorial = driver({
			showProgress: true,
			allowClose: true,
			onPopoverRender: (popover, { config, state }) => {
				if (state.activeIndex == 0) {
					const skipThisButton = document.createElement('button')
					skipThisButton.innerText = 'Skip this tutorial'
					skipThisButton.addEventListener('click', () => {
						updateProgress(3)

						branchAllTutorial.destroy()
					})
					skipThisButton.setAttribute(
						'class',
						'border px-2 py-1 !text-xs font-normal rounded-md hover:bg-surface-hover transition-all flex items-center'
					)

					const skipAllButton = document.createElement('button')
					skipAllButton.innerText = 'Skip all tutorials'
					skipAllButton.addEventListener('click', () => {
						dispatch('skipAll')
						branchAllTutorial.destroy()
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
					element: '#flow-editor-add-step-0',
					popover: {
						title: 'Add a step',
						description: 'Click here to add a step to your flow',
						onNextClick: () => {
							triggerAddFlowStep(0)

							setTimeout(() => {
								branchAllTutorial.moveNext()
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
							selectFlowStepKind(6)

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
						description: 'Click here to add a branch',
						onNextClick: () => {
							clickButtonBySelector('#add-branch-button')

							setTimeout(() => {
								branchAllTutorial.moveNext()
							})
						}
					}
				},

				{
					element: '#flow-editor-step-input',
					popover: {
						title: 'Branch all',
						description:
							'All branches will be executed, and the result will be gathered in an array',

						onNextClick: () => {
							if ($flowStore.value.modules[0].value.type === 'branchall') {
								$flowStore.value.modules[0].value = {
									type: 'branchall',
									branches: [
										{
											modules: [
												{
													id: 'b',
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
													id: 'c',
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
							}

							$flowStore = $flowStore
							dispatch('reload')

							setTimeout(() => {
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
							clickButtonBySelector('#flow-editor-test-flow')

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
							clickButtonBySelector('#flow-editor-test-flow-drawer')

							setTimeout(() => {
								branchAllTutorial.moveNext()
								updateProgress(3)
							})
						}
					}
				}
			]
		})
		branchAllTutorial.drive()
	}
</script>

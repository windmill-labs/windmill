<script lang="ts">
	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import TutorialItem from './TutorialItem.svelte'
	import { clickButtonBySelector, triggerAddFlowStep, selectFlowStepKind } from './utils'
	import { updateProgress } from '$lib/tutorialUtils'
	import { RawScript } from '$lib/gen'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()

	function runTutorial() {
		const branchAllTutorial = driver({
			showProgress: true,
			allowClose: true,
			steps: [
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
						title: 'Insert loop',
						description: "Let's pick forloop",
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
						description: 'Click here to add a branch to your loop',
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
								updateProgress(1)
							})
						}
					}
				}
			]
		})
		branchAllTutorial.drive()
	}
</script>

<TutorialItem on:click={() => runTutorial()} label="Branch all tutorial" index={1} />

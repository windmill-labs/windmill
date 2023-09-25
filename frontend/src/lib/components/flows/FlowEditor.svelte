<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, tick } from 'svelte'
	import type { FlowEditorContext } from './types'
	import type { FlowCopilotContext } from '../copilot/flow'
	import { classNames } from '$lib/utils'

	import { driver } from 'driver.js'
	import 'driver.js/dist/driver.css'

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	const driverObj = driver({
		showProgress: true,
		allowClose: true,
		steps: [
			/*
			{
				popover: {
					title: 'Welcome in the Windmil Flow editor',
					description: 'Learn how to build powerfull flows in a few steps'
				}
			},

			{
				popover: {
					title: 'Flows inputs',
					description: 'Flows have inputs that can be used in the flow'
				},
				element: '#svelvet-Input'
			},
			{
				element: '#flow-editor-add-property',
				popover: {
					title: 'Add a property',
					description: 'Click here to add a property to your schema',
					onNextClick: () => {
						const button = document.querySelector('#flow-editor-add-property') as HTMLButtonElement
						if (button) {
							button.click()
						}
						setTimeout(() => {
							driverObj.moveNext()
						})
					}
				}
			},
			{
				element: '#schema-modal-name',
				popover: {
					title: 'Name your property',
					description: 'Give a name to your property',
					onNextClick: () => {
						const input = document.querySelector('#schema-modal-name') as HTMLInputElement

						if (input) {
							input.value = 'Hello World'
							input.dispatchEvent(new Event('input', { bubbles: true }))
						}
						driverObj.moveNext()
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
							driverObj.moveNext()
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
						const button = document.querySelector('#flow-editor-add-step-0') as HTMLButtonElement
						if (button) {
							button.parentElement?.dispatchEvent(
								new PointerEvent('pointerdown', { bubbles: true })
							)
						}
						setTimeout(() => {
							driverObj.moveNext()
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
					title: '2',
					description: 'Description',
					onNextClick: () => {
						const button = document.querySelector(
							'#flow-editor-insert-module > div > button:nth-child(1)'
						) as HTMLButtonElement

						if (button) {
							button?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
						}

						setTimeout(() => {
							driverObj.moveNext()
						})
					}
				},
				element: '#flow-editor-insert-module > div > button:nth-child(1)'
			},
			{
				element: '#flow-editor-flow-inputs',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow'
				}
			},
			{
				element: '#flow-editor-action-script',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow'
				}
			},
			{
				element: '#flow-editor-action-script > button:nth-child(1)',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						const button = document.querySelector(
							'#flow-editor-action-script > button > div > button:nth-child(1)'
						) as HTMLButtonElement

						if (button) {
							button?.click()
						}

						setTimeout(() => {
							driverObj.moveNext()
						})
					}
				}
			},
			*/
			{
				element: '#flow-editor-add-step-1',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						const button = document.querySelector('#flow-editor-add-step-1') as HTMLButtonElement
						if (button) {
							button.parentElement?.dispatchEvent(
								new PointerEvent('pointerdown', { bubbles: true })
							)
						}
						setTimeout(() => {
							driverObj.moveNext()
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
					description: 'Description',
					onNextClick: () => {
						const button = document.querySelector(
							'#flow-editor-insert-module > div > button:nth-child(3)'
						) as HTMLButtonElement

						if (button) {
							button?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
						}

						setTimeout(() => {
							driverObj.moveNext()
						})
					}
				},
				element: '#flow-editor-insert-module > div > button:nth-child(3)'
			},

			{
				element: '#flow-editor-iterator-expression',
				popover: {
					title: '',
					description: 'Description',
					onNextClick: () => {
						if ($flowStore.value.modules[1].value.type === 'forloopflow') {
							if ($flowStore.value.modules[1].value.iterator.type === 'javascript') {
								$flowStore.value.modules[1].value.iterator.expr = 'results.a.split("")'
							}
						}

						$flowStore = $flowStore

						renderCount += 1

						tick().then(() => {
							driverObj.moveNext()
						})
					}
				}
			},
			{
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						if ($flowStore.value.modules[1].value.type === 'forloopflow') {
							$flowStore.value.modules[1].value.modules = [
								{
									id: 'c',
									value: {
										type: 'rawscript',
										content: 'def main(letter: str):\n    return letter.capitalize()',
										// @ts-ignore
										language: 'python3',
										input_transforms: {
											letter: {
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

						$flowStore = $flowStore

						renderCount += 1

						tick().then(() => {
							driverObj.moveNext()
						})
					}
				}
			},
			{
				element: '#svelvet-c',
				popover: {
					title: 'Add a step',
					description: 'Click here to add a step to your flow',
					onNextClick: () => {
						$selectedId = 'c'
						renderCount += 1
						tick().then(() => {
							driverObj.moveNext()
						})
					}
				}
			}
		]
	})

	driverObj.drive()

	export let loading: boolean

	let size = 40
	let renderCount = 1

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')
</script>

{#key renderCount}
	<div
		class={classNames(
			'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t',
			$copilotCurrentStepStore !== undefined ? 'border-gray-500/75' : ''
		)}
	>
		<Splitpanes>
			<Pane {size} minSize={15} class="h-full relative z-0">
				<div class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative">
					{#if loading}
						<div class="p-2 pt-10">
							{#each new Array(6) as _}
								<Skeleton layout={[[2], 1.5]} />
							{/each}
						</div>
					{:else if $flowStore.value.modules}
						<FlowModuleSchemaMap bind:modules={$flowStore.value.modules} />
					{/if}
				</div>
			</Pane>
			<Pane class="relative z-10" size={100 - size} minSize={40}>
				{#if loading}
					<div class="w-full h-full">
						<div class="block m-auto mt-40 w-10">
							<WindmillIcon height="40px" width="40px" spin="fast" />
						</div>
					</div>
				{:else}
					<FlowEditorPanel />
				{/if}
			</Pane>
		</Splitpanes>
	</div>
{/key}

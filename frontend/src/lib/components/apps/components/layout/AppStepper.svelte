<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import { classNames } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { RunnableComponent, RunnableWrapper } from '../helpers'
	import type { AppInput } from '../../inputType'

	export let id: string
	export let configuration: RichConfigurations
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'steppercomponent'> | undefined = undefined
	export let render: boolean
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let componentInput: AppInput | undefined

	let resolvedConfig = initConfig(
		components['steppercomponent'].initialData.configuration,
		configuration
	)

	$: statusByStep = [] as Array<'success' | 'error' | 'pending'>

	const { app, worldStore, focusedGrid, selectedComponent, componentControl, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	let selected: string = tabs[0]
	let tabHeight: number = 0
	let footerHeight: number = 0
	let runnableComponent: RunnableComponent
	let selectedIndex = tabs?.indexOf(selected) ?? -1
	let maxReachedIndex = -1

	let outputs = initOutput($worldStore, id, {
		currentStepIndex: 0,
		result: undefined,
		loading: false,
		final: false,
		shouldValidate: false
	})

	function handleTabSelection() {
		if (selectedIndex > maxReachedIndex) {
			maxReachedIndex = selectedIndex
		}

		selectedIndex = tabs?.indexOf(selected)
		outputs?.currentStepIndex.set(selectedIndex)
		outputs?.final.set(tabs.length - 1 === selectedIndex)

		if ($focusedGrid?.parentComponentId != id || $focusedGrid?.subGridIndex != selectedIndex) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedIndex
			}
		}
	}

	function setShouldValidate(shouldValidate: boolean) {
		outputs?.shouldValidate.set(shouldValidate)
	}

	function getStepColor(index: number, selectedIndex: number, maxReachedIndex: number) {
		if (index === selectedIndex) {
			return 'bg-blue-500 text-white'
		} else if (index > maxReachedIndex) {
			return 'bg-gray-200'
		} else {
			return 'bg-blue-200'
		}
	}

	function getValidationStepColor(
		statusByStep: Array<'success' | 'error' | 'pending'>,
		index: number,
		selectedIndex: number
	) {
		const status = statusByStep[index]

		if (index === selectedIndex) {
			return 'bg-blue-500 text-white'
		} else if (status === 'success') {
			return 'bg-green-500 text-white'
		} else if (status === 'error') {
			return 'bg-red-500 text-white'
		} else {
			return 'bg-gray-200'
		}
	}

	let result: { error: { name: string; message: string; stack: string } } | undefined = undefined

	async function runStep() {
		statusByStep[selectedIndex] = 'pending'

		if (lastStep || resolvedConfig.shouldValidate) {
			if (runnableComponent) {
				await runnableComponent?.runComponent()
			}
		}

		if (result?.error) {
			statusByStep[selectedIndex] = 'error'
		} else {
			statusByStep[selectedIndex] = 'success'

			if (!lastStep) {
				selected = tabs[selectedIndex + 1]
			}
		}
	}

	$: resolvedConfig.shouldValidate && setShouldValidate(resolvedConfig.shouldValidate)

	$componentControl[id] = {
		left: () => {
			const index = tabs.indexOf(selected)
			if (index > 0) {
				selected = tabs[index - 1]
				return true
			}
			return false
		},
		right: () => {
			const index = tabs.indexOf(selected)
			if (index < tabs.length - 1) {
				selected = tabs[index + 1]
				return true
			}
			return false
		},
		setTab: (tab: number) => {
			selected = tabs[tab]
			handleTabSelection()
		}
	}

	$: selected != undefined && handleTabSelection()

	$: css = concatCustomCss($app.css?.steppercomponent, customCss)
	$: lastStep = selectedIndex === tabs.length - 1
</script>

<InputValue {id} input={configuration.shouldValidate} bind:value={resolvedConfig.shouldValidate} />
<InitializeComponent {id} />
<RunnableWrapper
	{recomputeIds}
	{render}
	bind:runnableComponent
	{componentInput}
	{id}
	{extraQueryParams}
	autoRefresh={false}
	forceSchemaDisplay={true}
	runnableClass="!block"
	{outputs}
	triggerable
	bind:result
>
	<div class="w-full">
		<div bind:clientHeight={tabHeight}>
			<div class="flex justify-between">
				<ol
					class={twMerge(
						'relative z-20 flex justify-between items-centers text-sm font-medium text-gray-500',
						css?.selectedTab?.class
					)}
					style={css?.tabRow?.style}
				>
					{#each tabs ?? [] as step, index}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<li
							class="flex items-center gap-2 px-2 py-1 cursor-pointer hover:bg-gray-100 rounded-md"
							on:click={() => {
								selected = step
							}}
						>
							<span
								class={classNames(
									'h-6 w-6 rounded-full text-center text-[10px]/6 font-bold flex items-center justify-center',
									resolvedConfig.shouldValidate
										? getValidationStepColor(statusByStep, index, selectedIndex)
										: getStepColor(index, selectedIndex, maxReachedIndex)
								)}
								class:font-bold={selectedIndex === index}
							>
								{index + 1}
							</span>

							<span class="hidden sm:block">{step}</span>
						</li>
						{#if index !== (tabs ?? []).length - 1}
							<li class="flex items-center">
								<div class="h-0.5 w-4 bg-blue-200" />
							</li>
						{/if}
					{/each}
				</ol>
			</div>
		</div>

		<div class="w-full">
			{#if $app.subgrids}
				{#each tabs ?? [] as _res, i}
					<SubGridEditor
						{id}
						visible={render && i === selectedIndex}
						subGridId={`${id}-${i}`}
						class={css?.container?.class}
						style={css?.container?.style}
						containerHeight={componentContainerHeight - tabHeight - footerHeight}
						on:focus={() => {
							if (!$connectingInput.opened) {
								$selectedComponent = [id]
								handleTabSelection()
							}
						}}
					/>
				{/each}
			{/if}
		</div>

		<div bind:clientHeight={footerHeight}>
			<div class="flex justify-between h-10 p-2">
				<div class="flex items-center gap-2">
					<span class="text-sm font-medium text-gray-500">
						Step {selectedIndex + 1} of {tabs.length}
					</span>
				</div>
				<div class="flex items-center gap-2">
					<Button
						size="xs"
						color="light"
						variant="border"
						disabled={selectedIndex === 0}
						on:click={() => {
							selected = tabs[selectedIndex - 1]
						}}
					>
						Previous
					</Button>
					<Button
						size="xs"
						color={lastStep ? 'dark' : 'light'}
						variant={lastStep ? 'contained' : 'border'}
						on:click={runStep}
					>
						{#if lastStep}
							Submit
						{:else}
							Next
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</div>
</RunnableWrapper>

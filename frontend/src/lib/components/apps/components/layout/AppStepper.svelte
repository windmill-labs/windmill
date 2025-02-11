<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { AppInput } from '../../inputType'
	import { ArrowLeftIcon, ArrowRightIcon, Loader2 } from 'lucide-svelte'
	import Stepper from '$lib/components/common/stepper/Stepper.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'steppercomponent'> | undefined = undefined
	export let render: boolean
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let componentInput: AppInput | undefined
	export let onNext: string[] | undefined = undefined
	export let onPrevious: string[] | undefined = undefined

	const {
		app,
		worldStore,
		focusedGrid,
		selectedComponent,
		componentControl,
		connectingInput,
		mode,
		runnableComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	let everRender = render
	$: render && !everRender && (everRender = true)

	let selected = tabs[0]
	let tabHeight: number = 0
	let footerHeight: number = 0
	let runnableComponent: RunnableComponent
	let selectedIndex = tabs?.indexOf(selected) ?? -1
	let maxReachedIndex = -1
	let statusByStep = [] as Array<'success' | 'error' | 'pending'>
	let debugMode: boolean = false

	let outputs = initOutput($worldStore, id, {
		currentStepIndex: 0,
		result: undefined,
		loading: false,
		lastAction: undefined as 'previous' | 'next' | undefined
	})

	async function handleTabSelection() {
		if (runnableComponent && !debugMode) {
			await runnableComponent?.runComponent()
		}

		selectedIndex = tabs?.indexOf(selected)
		if (selectedIndex > maxReachedIndex) {
			maxReachedIndex = selectedIndex
		}
		outputs?.currentStepIndex.set(selectedIndex)

		if ($focusedGrid?.parentComponentId != id || $focusedGrid?.subGridIndex != selectedIndex) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedIndex
			}
		}
	}

	let result: { error: { name: string; message: string; stack: string } } | undefined = undefined

	async function runStep(targetIndex: number) {
		statusByStep[selectedIndex] = 'pending'

		outputs?.lastAction.set(directionClicked === 'left' ? 'previous' : 'next')

		if (runnableComponent && !debugMode) {
			await runnableComponent?.runComponent()
		}

		if (result?.error !== undefined) {
			statusByStep[selectedIndex] = 'error'
		} else {
			statusByStep[selectedIndex] = 'success'

			selected = tabs[targetIndex]
		}
		directionClicked = undefined
	}

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
			debugMode = tab >= 0

			if (debugMode) {
				selected = tabs[tab]
			} else {
				selected = tabs[0]
			}

			handleTabSelection()
		}
	}

	$: selected != undefined && handleTabSelection()
	let css = initCss($app.css?.steppercomponent, customCss)
	$: lastStep = selectedIndex === tabs.length - 1

	let directionClicked: 'left' | 'right' | undefined = undefined
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.steppercomponent}
	/>
{/each}

<InitializeComponent {id} />
<RunnableWrapper
	hasChildrens
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
	bind:result
	errorHandledByComponent={true}
>
	{#if everRender}
		<div class="w-full overflow-auto">
			{#if render}
				<div bind:clientHeight={tabHeight}>
					<Stepper
						on:click={(e) => {
							const index = e.detail.index
							if (index <= maxReachedIndex || $mode === 'dnd') {
								runStep(index)
							}
						}}
						{tabs}
						{selectedIndex}
						{maxReachedIndex}
						{statusByStep}
						hasValidations={Boolean(runnableComponent)}
					/>
				</div>
			{/if}

			<div class="w-full">
				{#if $app.subgrids}
					{#each tabs ?? [] as _res, i}
						<SubGridEditor
							{id}
							visible={render && i === selectedIndex}
							subGridId={`${id}-${i}`}
							class={twMerge(css?.container?.class, 'wm-stepper')}
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

			{#if render}
				<div bind:clientHeight={footerHeight}>
					<div class="flex justify-between h-10 p-2">
						<div class="flex items-center gap-2">
							<span class="text-sm font-medium text-tertiary">
								Step {selectedIndex + 1} of {tabs.length}
							</span>
						</div>
						<div class="flex items-center gap-2">
							<Button
								size="xs"
								color="light"
								variant="contained"
								disabled={selectedIndex === 0}
								on:click={(e) => {
									e.preventDefault()
									directionClicked = 'left'
									onPrevious?.forEach((id) =>
										$runnableComponents?.[id]?.cb?.forEach((cb) => cb?.())
									)
									runStep(selectedIndex - 1)
								}}
							>
								<div class="flex flex-row gap-2">
									{#if statusByStep[selectedIndex] === 'pending' && directionClicked === 'left'}
										<Loader2 class="w-4 h-4 animate-spin" />
									{:else}
										<ArrowLeftIcon class="w-4 h-4" />
									{/if}
									Previous
								</div>
							</Button>

							<Button
								size="xs"
								color="dark"
								variant="contained"
								disabled={lastStep}
								on:click={(e) => {
									e.preventDefault()
									directionClicked = 'right'
									onNext?.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb?.()))
									runStep(selectedIndex + 1)
								}}
							>
								<div class="flex flex-row gap-2">
									Next
									{#if statusByStep[selectedIndex] === 'pending' && directionClicked === 'right'}
										<Loader2 class="w-4 h-4 animate-spin" />
									{:else}
										<ArrowRightIcon class="w-4 h-4" />
									{/if}
								</div>
							</Button>
						</div>
					</div>
				</div>
			{/if}
		</div>
	{:else if $app.subgrids}
		{#each tabs ?? [] as _res, i}
			<SubGridEditor {id} visible={false} subGridId={`${id}-${i}`} />
		{/each}
	{/if}
</RunnableWrapper>

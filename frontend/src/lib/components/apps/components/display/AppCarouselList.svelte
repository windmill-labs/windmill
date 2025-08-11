<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import type { AppInput } from '../../inputType'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ListWrapper from '../layout/ListWrapper.svelte'
	import Carousel from 'svelte-carousel'
	import { ArrowLeftCircle, ArrowRightCircle } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'carousellistcomponent'> | undefined
		render: boolean
		initializing: boolean | undefined
		componentContainerHeight: number
	}

	let {
		id,
		componentInput,
		configuration,
		customCss = undefined,
		render,
		initializing = $bindable(),
		componentContainerHeight
	}: Props = $props()

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		inputs: {},
		currentIndex: 0
	})

	const resolvedConfig = $state(
		initConfig(components['carousellistcomponent'].initialData.configuration, configuration)
	)

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	let css = $state(initCss($app.css?.carousellistcomponent, customCss))
	let result: any[] | undefined = $state(undefined)

	let inputs = $state({})
	let carousel: Carousel = $state()

	$effect.pre(() => {
		$selectedComponent?.includes(id) &&
			$focusedGrid === undefined &&
			($focusedGrid = {
				parentComponentId: id,
				subGridIndex: 0
			})
	})
	let currentPageIndex = $state(0)

	// Single update function - ONLY place that calls .set()
	function handleIndexChange() {
		if (outputs?.currentIndex) {
			if (!Array.isArray(result) || result.length === 0) {
				// No data or empty data - reset to 0
				currentPageIndex = 0
				outputs.currentIndex.set(0)
			} else if (currentPageIndex >= result.length) {
				// Current index is out of bounds - reset to 0
				currentPageIndex = 0
				outputs.currentIndex.set(0)
			} else {
				// Valid data and valid current index
				outputs.currentIndex.set(currentPageIndex)
				// Navigate carousel to match the current index
				if (carousel) {
					carousel.goTo(currentPageIndex)
				}
			}
		}
	}

	// Watch for changes and call update function (like working components)
	$effect.pre(() => {
		currentPageIndex != undefined && untrack(() => handleIndexChange())
	})

	$componentControl[id] = {
		setSelectedIndex: (index: number) => {
			if (Array.isArray(result) && index >= 0 && index < result.length) {
				currentPageIndex = index
			}
		}
	}
</script>

{#each Object.keys(components['carousellistcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.carousellistcomponent}
	/>
{/each}

<InitializeComponent {id} />

<RunnableWrapper {render} {outputs} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if everRender}
		<div class="w-full flex flex-wrap overflow-auto divide-y max-h-full">
			{#if $app.subgrids?.[`${id}-0`]}
				{#if Array.isArray(result) && result.length > 0}
					{#key result}
						<Carousel
							particlesToShow={1}
							particlesToScroll={1}
							autoplay={false}
							autoplayProgressVisible={false}
							timingFunction={resolvedConfig.timingFunction}
							dots={true}
							arrows={true}
							swiping={false}
							bind:this={carousel}
							on:pageChange={(event) => {
								currentPageIndex = event.detail
								$focusedGrid = {
									parentComponentId: id,
									subGridIndex: event.detail
								}
							}}
						>
							{#snippet prev()}
								<div class="h-full flex justify-center flex-col p-2">
									<div>
										<Button
											color="light"
											on:click={() => {
												const pagesCount = result?.length ?? 0

												if (currentPageIndex > 0) {
													carousel.goTo(currentPageIndex - 1)
												} else {
													carousel.goTo(pagesCount - 1)
												}
											}}
										>
											<ArrowLeftCircle size={16} />
										</Button>
									</div>
								</div>
							{/snippet}
							{#snippet next()}
								<div class="h-full flex justify-center flex-col p-2">
									<div>
										<Button
											color="light"
											on:click={() => {
												const pagesCount = result?.length ?? 0
												if (currentPageIndex < pagesCount - 1) {
													carousel.goTo(currentPageIndex + 1)
												} else {
													carousel.goTo(0)
												}
											}}
										>
											<ArrowRightCircle size={16} />
										</Button>
									</div>
								</div>
							{/snippet}
							{#each result ?? [] as value, index}
								<div class="overflow-auto w-full">
									<ListWrapper
										onSet={(id, value) => {
											if (!inputs[id]) {
												inputs[id] = { [index]: value }
											} else {
												inputs[id] = { ...inputs[id], [index]: value }
											}
											outputs?.inputs.set(inputs, true)
										}}
										onRemove={(id) => {
											if (inputs?.[id] == undefined) {
												return
											}
											if (index == 0) {
												delete inputs[id]
												inputs = { ...inputs }
											} else {
												delete inputs[id][index]
												inputs[id] = { ...inputs[id] }
											}
											outputs?.inputs.set(inputs, true)
										}}
										{value}
										{index}
									>
										<SubGridEditor
											{id}
											visible={render}
											class={twMerge(css?.container?.class, 'wm-carousel')}
											style={css?.container?.style}
											subGridId={`${id}-0`}
											containerHeight={componentContainerHeight - 40}
											on:focus={() => {
												if (!$connectingInput.opened) {
													$selectedComponent = [id]
												}
												onFocus()
											}}
										/>
									</ListWrapper>
								</div>
							{/each}
						</Carousel>
					{/key}
				{:else}
					<ListWrapper disabled value={undefined} index={0}>
						<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
					</ListWrapper>
					{#if !Array.isArray(result)}
						<div class="text-center text-tertiary">Input data is not an array</div>
					{/if}
				{/if}
			{/if}
		</div>
	{:else if $app.subgrids}
		<ListWrapper disabled value={undefined} index={0}>
			<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
		</ListWrapper>
	{/if}
</RunnableWrapper>

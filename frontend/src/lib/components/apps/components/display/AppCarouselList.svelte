<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import type { AppInput } from '../../inputType'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ListWrapper from '../layout/ListWrapper.svelte'
	import Carousel from 'svelte-carousel'
	import { ArrowLeftCircle, ArrowRightCircle } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean
	export let initializing: boolean | undefined
	export let componentContainerHeight: number

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		inputs: {}
	})

	const resolvedConfig = initConfig(
		components['carousellistcomponent'].initialData.configuration,
		configuration
	)

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
	let result: any[] | undefined = undefined

	let inputs = {}
	let carousel: Carousel
</script>

{#each Object.keys(components['carousellistcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<RunnableWrapper
	render={true}
	{outputs}
	autoRefresh
	{componentInput}
	{id}
	bind:initializing
	bind:result
>
	<div class="w-full flex flex-wrap overflow-auto divide-y max-h-full">
		{#if $app.subgrids?.[`${id}-0`]}
			{#if Array.isArray(result) && result.length > 0}
				<Carousel
					particlesToShow={resolvedConfig.particlesToShow}
					particlesToScroll={resolvedConfig.particlesToScroll}
					autoplay={false}
					autoplayProgressVisible={false}
					timingFunction={resolvedConfig.timingFunction}
					dots={true}
					arrows={true}
					swiping={false}
					on:pageChange={(event) => {
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: event.detail
						}
					}}
					bind:this={carousel}
					let:currentPageIndex
				>
					<div slot="prev" class="h-full flex justify-center flex-col p-2">
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
					<div slot="next" class="h-full flex justify-center flex-col p-2">
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
					{#each result ?? [] as value, index}
						<div class="overflow-auto w-full">
							<ListWrapper
								on:inputsChange={() => {
									outputs?.inputs.set(inputs, true)
								}}
								bind:inputs
								{value}
								{index}
							>
								<SubGridEditor
									{id}
									visible={render}
									class={css?.container?.class}
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
</RunnableWrapper>

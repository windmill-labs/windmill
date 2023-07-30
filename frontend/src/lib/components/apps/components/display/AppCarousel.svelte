<script lang="ts">
	import Carousel from 'svelte-carousel'
	import { getContext } from 'svelte'

	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { initConfig } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { onMount } from 'svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'carouselcomponent'> | undefined = undefined
	export let componentContainerHeight: number

	const resolvedConfig = initConfig(
		components['carouselcomponent'].initialData.configuration,
		configuration
	)

	const { app, selectedComponent, connectingInput, focusedGrid, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	$: css = concatCustomCss($app.css?.carouselcomponent, customCss)

	$: $selectedComponent?.includes(id) &&
		$focusedGrid === undefined &&
		($focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		})

	let width = 0
	function getWidthOfPagesWindow() {
		const elements = document.getElementsByClassName('sc-carousel__pages-window')

		if (elements.length > 0) {
			width = elements[0].getBoundingClientRect().width
		}
	}

	onMount(getWidthOfPagesWindow)
</script>

{#each Object.keys(components['carouselcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

{#key tabs.join()}
	<div class="w-full" style={`height: ${componentContainerHeight - 2}`}>
		{#if tabs.length > 0}
			<Carousel
				particlesToShow={resolvedConfig.particlesToShow}
				particlesToScroll={resolvedConfig.particlesToScroll}
				autoplay={resolvedConfig.autoplay}
				autoplayDuration={resolvedConfig.autoplayDuration}
				autoplayProgressVisible={resolvedConfig.autoplayProgressVisible}
				pauseOnFocus={resolvedConfig.pauseOnFocus}
				timingFunction={resolvedConfig.timingFunction}
				dots={resolvedConfig.dots}
				arrows={resolvedConfig.arrows}
				swiping={$mode === 'preview' ? resolvedConfig.swiping : false}
				on:pageChange={(event) => {
					$focusedGrid = {
						parentComponentId: id,
						subGridIndex: event.detail
					}
				}}
			>
				{#if $app.subgrids}
					{#each tabs ?? [] as _res, i}
						<SubGridEditor
							{id}
							visible={true}
							subGridId={`${id}-${i}`}
							class={css?.container?.class}
							style={css?.container?.style}
							containerHeight={resolvedConfig.dots
								? componentContainerHeight - 40
								: componentContainerHeight}
							containerWidth={width}
							on:focus={() => {
								if (!$connectingInput.opened) {
									$selectedComponent = [id]
								}

								$focusedGrid = {
									parentComponentId: id,
									subGridIndex: i
								}
							}}
						/>
					{/each}
				{/if}
			</Carousel>
		{/if}
	</div>
{/key}

<style lang="postcss">
	:global(.sc-carousel-arrow__circle) {
		background-color: rgb(var(--color-surface-inverse) / var(--tw-bg-opacity)) !important;
	}

	:global(.sc-carousel-arrow__arrow) {
		--tw-border-opacity: 1 !important;
		border-color: rgb(var(--color-text-primary-inverse) / var(--tw-border-opacity)) !important;
	}

	:global(.sc-carousel-dot__dot) {
		background-color: rgb(var(--color-surface-inverse) / var(--tw-bg-opacity)) !important;
	}
</style>

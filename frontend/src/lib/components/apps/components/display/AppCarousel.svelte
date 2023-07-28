<script lang="ts">
	import Carousel from 'svelte-carousel'

	import { getContext } from 'svelte'
	import { initConfig } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let componentContainerHeight: number
	export let tabs: string[]
	export let customCss: ComponentCustomCSS<'carouselcomponent'> | undefined = undefined
	export let render: boolean

	let resolvedConfig = initConfig(
		components['carouselcomponent'].initialData.configuration,
		configuration
	)

	const { app, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	$: css = concatCustomCss($app.css?.carouselcomponent, customCss)
</script>

<InputValue {id} input={configuration.tabsKind} bind:value={resolvedConfig.tabsKind} />

<InitializeComponent {id} />

<div class={resolvedConfig.tabsKind == 'sidebar' ? 'flex gap-4 w-full' : 'w-full'}>
	<div class="w-full">
		<Carousel>
			{#if $app.subgrids}
				{#each tabs ?? [] as _res, i}
					<SubGridEditor
						{id}
						visible={render}
						subGridId={`${id}-${i}`}
						class={css?.container?.class}
						style={css?.container?.style}
						containerHeight={componentContainerHeight}
						on:focus={() => {
							if (!$connectingInput.opened) {
								$selectedComponent = [id]
							}
						}}
					/>
				{/each}
			{/if}
		</Carousel>
	</div>
</div>

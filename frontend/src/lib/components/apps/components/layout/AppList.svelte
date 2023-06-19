<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ListWrapper from './ListWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean
	export let initializing: boolean | undefined

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let resolvedConfig = initConfig(
		components['listcomponent'].initialData.configuration,
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
</script>

{#each Object.keys(components['listcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />
<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div class="w-full h-full flex flex-wrap overflow-auto gap-2">
		{#if $app.subgrids?.[`${id}-0`]}
			{#each result ?? [] as value, index}
				<div
					style={`min-width: ${resolvedConfig.minWidthPx}px; max-height: ${resolvedConfig.heightPx}px;`}
					class="border overflow-auto"
				>
					<ListWrapper {value} {index}>
						<SubGridEditor
							visible={render}
							{id}
							class={css?.container?.class}
							style={css?.container?.style}
							subGridId={`${id}-0`}
							containerHeight={resolvedConfig.heightPx}
							forceView={index != 0}
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
		{/if}
	</div>
</RunnableWrapper>

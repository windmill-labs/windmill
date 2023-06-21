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

	$: isCard = resolvedConfig.width?.selected == 'card'
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

{#if render}
	<RunnableWrapper
		{outputs}
		{render}
		autoRefresh
		{componentInput}
		{id}
		bind:initializing
		bind:result
	>
		<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />

		<div
			class="w-full flex flex-wrap overflow-auto {isCard ? 'h-full gap-2' : 'divide-y max-h-full'}"
		>
			{#if $app.subgrids?.[`${id}-0`]}
				{#if Array.isArray(result) && result.length > 0}
					{#each result ?? [] as value, index}
						<div
							style={`${
								isCard
									? `min-width: ${resolvedConfig.width?.configuration?.card?.minWidthPx}px; `
									: ''
							} max-height: ${resolvedConfig.heightPx}px;`}
							class="overflow-auto {!isCard ? 'w-full' : 'border'}"
						>
							<ListWrapper {value} {index}>
								<SubGridEditor
									visible={render}
									{id}
									class={css?.container?.class}
									style={css?.container?.style}
									subGridId={`${id}-0`}
									containerHeight={resolvedConfig.heightPx}
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
				{:else}
					<ListWrapper disabled value={undefined} index={0}>
						<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
					</ListWrapper>
					{#if !Array.isArray(result)}
						<div class="text-center text-gray-500">Input data is not an array</div>
					{/if}
				{/if}
			{/if}
		</div>
	</RunnableWrapper>
{:else}
	<ListWrapper disabled value={undefined} index={0}>
		<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
	</ListWrapper>
{/if}

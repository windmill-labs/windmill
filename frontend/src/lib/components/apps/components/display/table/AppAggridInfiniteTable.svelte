<script lang="ts">
	import type { IDatasource } from 'ag-grid-community'

	import { getContext } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'

	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { components, type TableAction } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'

	import 'ag-grid-community/styles/ag-grid.css'
	import './theme/windmill-theme.css'

	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import AppAggridExplorerTable from './AppAggridExplorerTable.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let actions: TableAction[] | undefined = undefined

	const context = getContext<AppViewerContext>('AppViewerContext')
	const { app, worldStore } = context

	let css = initCss($app.css?.aggridcomponent, customCss)
	let result: any[] | undefined = undefined

	let resolvedConfig = initConfig(
		components['aggridinfinitecomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: {},
		selectedRows: [] as any[],
		result: [] as any[],
		inputs: {},
		loading: false,
		page: 0,
		newChange: { row: 0, column: '', value: undefined },
		ready: undefined as boolean | undefined
	})

	let loading: boolean = false

	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			if (result) {
				params.successCallback(result, result.length)
			}
		}
	}

	let renderCount = 0
</script>

{#each Object.keys(components['aggridcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.tablecomponent}
	/>
{/each}

<RunnableWrapper
	{outputs}
	{render}
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
	hideRefreshButton={true}
>
	{#key renderCount}
		<AppAggridExplorerTable
			{id}
			{datasource}
			{resolvedConfig}
			{customCss}
			{outputs}
			allowDelete={false}
			{actions}
		/>
	{/key}
</RunnableWrapper>

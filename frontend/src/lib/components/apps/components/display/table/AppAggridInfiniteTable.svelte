<script lang="ts">
	import { GridApi, createGrid, type IDatasource } from 'ag-grid-community'
	import { isObject, sendUserToast } from '$lib/utils'
	import { getContext, onDestroy } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../../types'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'

	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { components, type TableAction } from '$lib/components/apps/editor/component'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { deepEqual } from 'fast-equals'
	import RefreshButton from '$lib/components/apps/components/RefreshButton.svelte'

	import 'ag-grid-community/styles/ag-grid.css'
	import './theme/windmill-theme.css'

	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import AppAggridExplorerTable from './AppAggridExplorerTable.svelte'

	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

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

	let value: any[] = Array.isArray(result)
		? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() }))
		: [{ error: 'input was not an array' }]

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
			params.successCallback(value, value.length)
		}
	}
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
	<AppAggridExplorerTable
		{id}
		{datasource}
		{resolvedConfig}
		{customCss}
		{outputs}
		allowDelete={false}
		{actions}
	/>
</RunnableWrapper>

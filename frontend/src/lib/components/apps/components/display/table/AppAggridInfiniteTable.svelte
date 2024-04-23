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
	import type { RunnableComponent } from '../..'
	import { getPrimaryKeys } from '../dbtable/utils'
	import InitializeComponent from '../../helpers/InitializeComponent.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let actions: TableAction[] | undefined = undefined
	let runnableComponent: RunnableComponent | undefined = undefined

	const context = getContext<AppViewerContext>('AppViewerContext')
	const { app, worldStore } = context

	let css = initCss($app.css?.aggridcomponent, customCss)
	let result: any[] | undefined = undefined
	let loading: boolean = false

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
		ready: undefined as boolean | undefined,
		params: {
			offset: 0,
			limit: 10,
			order_by: resolvedConfig.columnDefs?.[0]?.field,
			is_desc: false
		}
	})

	const datasource: IDatasource = {
		rowCount: undefined,

		getRows: async function (params) {
			if (!render) {
				return
			}

			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				order_by: params.sortModel?.[0]?.colId ?? resolvedConfig.columnDefs?.[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc',
				startRow: params.startRow,
				endRow: params.endRow
			}

			outputs.params.set(currentParams)

			if (!runnableComponent && result) {
				params.successCallback(result, result.length)
			}

			runnableComponent?.runComponent(undefined, undefined, undefined, currentParams, {
				done: (items) => {
					let lastRow = -1

					if (datasource?.rowCount && datasource.rowCount <= params.endRow) {
						lastRow = datasource.rowCount
					}

					if (items && Array.isArray(items)) {
						let processedData = items.map((item) => {
							let primaryKeys = getPrimaryKeys(resolvedConfig.columnDefs)
							let o = {}
							primaryKeys.forEach((pk) => {
								o[pk] = item[pk]
							})
							item['__index'] = JSON.stringify(o)
							return item
						})

						if (items.length < params.endRow - params.startRow) {
							lastRow = params.startRow + items.length
						}

						datasource.rowCount = undefined

						params.successCallback(processedData, lastRow)
					} else {
						params.failCallback()
					}
				},
				cancel: () => {
					params.failCallback()
				},
				error: () => {
					params.failCallback()
				}
			})
		}
	}

	let firstRow: number = 0
	let lastRow: number = 0

	// update outputs wheb firstRow or lastRow changes
	$: {
		if (firstRow !== undefined && lastRow !== undefined) {
			const x = outputs.params.peak()
			x.startRow = firstRow
			x.endRow = lastRow
			outputs.params.set(x)
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

<InitializeComponent {id} />

<RunnableWrapper
	{outputs}
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
	bind:runnableComponent
	render={false}
	autoRefresh={false}
	allowConcurentRequests
	noInitialize
/>

<AppAggridExplorerTable
	{id}
	{datasource}
	{resolvedConfig}
	{customCss}
	{outputs}
	allowDelete={false}
	{actions}
	bind:firstRow
	bind:lastRow
/>

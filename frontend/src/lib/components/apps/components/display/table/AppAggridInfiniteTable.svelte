<script lang="ts">
	import type { IDatasource } from 'ag-grid-community'

	import { getContext, untrack } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'

	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { components, type TableAction } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import './theme/windmill-theme.css'

	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import AppAggridExplorerTable from './AppAggridExplorerTable.svelte'
	import { getPrimaryKeys } from '../dbtable/utils'
	import InitializeComponent from '../../helpers/InitializeComponent.svelte'
	import DebouncedInput from '../../helpers/DebouncedInput.svelte'
	import RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import { CancelablePromise } from '$lib/gen'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		render: boolean
		customCss?: ComponentCustomCSS<'aggridinfinitecomponent'> | undefined
		actions?: TableAction[] | undefined
		onChange?: string[] | undefined
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		render,
		customCss = undefined,
		actions = undefined,
		onChange = undefined
	}: Props = $props()

	let runnableComponent: RunnableComponent | undefined = $state(undefined)

	function clear() {
		setTimeout(() => {
			aggrid?.clearRows()
		}, 0)
	}

	const context = getContext<AppViewerContext>('AppViewerContext')
	const { app, worldStore } = context

	let css = $state(initCss($app.css?.aggridcomponent, customCss))
	let result: any[] | undefined = $state(undefined)
	let loading: boolean = $state(false)

	let resolvedConfig = $state(
		initConfig(components['aggridinfinitecomponent'].initialData.configuration, configuration)
	)

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: {},
		selectedRows: [] as any[],
		openedModalRow: {},
		result: [] as any[],
		inputs: {},
		loading: false,
		page: 0,
		newChange: { row: 0, column: '', value: undefined },
		ready: undefined as boolean | undefined,
		params: {
			offset: 0,
			limit: 10,
			orderBy: resolvedConfig.columnDefs?.[0]?.field,
			isDesc: false,
			search: ''
		}
	})

	let aggrid: AppAggridExplorerTable | undefined = $state(undefined)

	const datasource: IDatasource = $state({
		rowCount: undefined,

		getRows: async function (params) {
			if (!render) {
				return
			}

			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				orderBy: params.sortModel?.[0]?.colId ?? undefined,
				isDesc: params.sortModel?.[0]?.sort === 'desc',
				search: resolvedConfig.searchEnabled ? searchValue : ''
			}

			outputs.params.set(currentParams)

			if (!runnableComponent && result) {
				params.successCallback(result, result.length)
				return
			}

			if (!runnableComponent && !result) {
				params.successCallback([], 0)
				return
			}

			runnableComponent?.runComponent(undefined, undefined, undefined, currentParams, {
				onDone: (items) => {
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
				onCancel: () => {
					params.failCallback()
				},
				onError: () => {
					params.failCallback()
				}
			})
		}
	})

	let searchValue: string = $state('')

	function updateSearchInOutputs() {
		outputs.params.set({
			...outputs.params.peak(),
			search: searchValue
		})
		aggrid?.clearRows()
	}

	$effect(() => {
		searchValue !== undefined && untrack(() => updateSearchInOutputs())
	})

	let ignoreFirst = true
</script>

{#each Object.keys(components['aggridinfinitecomponent'].initialData.configuration) as key (key)}
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
	preventDefaultRefresh
	overrideCallback={() =>
		new CancelablePromise(async (resolve) => {
			if (ignoreFirst) {
				ignoreFirst = false
				resolve()
				return
			}
			clear()
			resolve()
		})}
	{render}
	autoRefresh={true}
	allowConcurentRequests
	on:argsChanged={() => {
		clear()
	}}
>
	<div class="flex flex-col h-full">
		{#if resolvedConfig.searchEnabled}
			<div class="flex flex-row w-full justify-between items-center h-12">
				<div class="grow max-w-[300px]">
					<DebouncedInput placeholder="Search..." bind:value={searchValue} parentClass="h-full " />
				</div>
			</div>
		{/if}
		<AppAggridExplorerTable
			{id}
			{datasource}
			{resolvedConfig}
			{customCss}
			{outputs}
			{result}
			{actions}
			allowDelete={false}
			{onChange}
			bind:this={aggrid}
		/>
	</div>
</RunnableWrapper>

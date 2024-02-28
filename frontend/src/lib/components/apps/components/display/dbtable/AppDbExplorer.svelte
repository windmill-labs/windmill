<script lang="ts">
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		OneOfConfiguration,
		RichConfigurations
	} from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { findGridItem, initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import {
		getDbSchemas,
		loadTableMetaData,
		type TableMetadata,
		getPrimaryKeys,
		type ColumnDef
	} from './utils'
	import { getContext, tick } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore, type DBSchemas, type DBSchema } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import InsertRow from './InsertRow.svelte'
	import Portal from 'svelte-portal'
	import { sendUserToast } from '$lib/toast'
	import type { AppInput, StaticInput } from '$lib/components/apps/inputType'
	import DbExplorerCount from './DbExplorerCount.svelte'
	import AppAggridExplorerTable from '../table/AppAggridExplorerTable.svelte'
	import type { IDatasource } from 'ag-grid-community'
	import { RunnableWrapper } from '../../helpers'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import InsertRowRunnable from './InsertRowRunnable.svelte'
	import DeleteRow from './DeleteRow.svelte'
	import InitializeComponent from '../../helpers/InitializeComponent.svelte'
	import { getSelectInput } from './queries/select'
	import type { Preview } from '$lib/gen'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'dbexplorercomponent'> | undefined = undefined
	export let render: boolean
	export let initializing: boolean = true

	const resolvedConfig = initConfig(
		components['dbexplorercomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore, mode, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')
	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	let input: AppInput | undefined = undefined
	let quicksearch = ''
	let aggrid: AppAggridExplorerTable

	$: computeInput(
		resolvedConfig.columnDefs,
		resolvedConfig.whereClause,
		resolvedConfig.type.configuration[resolvedConfig.type.selected].resource
	)

	let timeoutInput: NodeJS.Timeout | undefined = undefined
	function computeInput(columnDefs: any, whereClause: string | undefined, resource: any) {
		if (timeoutInput) {
			clearTimeout(timeoutInput)
		}
		timeoutInput = setTimeout(() => {
			timeoutInput = undefined
			console.log('compute input')
			aggrid?.clearRows()
			input = getSelectInput(
				resource,
				resolvedConfig.type.configuration[resolvedConfig.type.selected].table,
				columnDefs,
				whereClause,
				resolvedConfig.type.selected as Preview.language
			)
		}, 1000)
	}

	$: editorContext != undefined && $mode == 'dnd' && resolvedConfig.type && listTableIfAvailable()

	$: editorContext != undefined &&
		$mode == 'dnd' &&
		resolvedConfig.type.configuration?.[resolvedConfig?.type?.selected]?.table &&
		listColumnsIfAvailable()

	let firstQuicksearch = true
	$: if (quicksearch) {
		if (firstQuicksearch) {
			firstQuicksearch = false
		} else {
			aggrid?.clearRows()
		}
	}

	initializing = false

	let updateCell: UpdateCell

	let renderCount = 0
	let insertDrawer: Drawer | undefined = undefined
	let componentContainerHeight: number | undefined = undefined
	let buttonContainerHeight: number | undefined = undefined

	function onUpdate(
		e: CustomEvent<{
			row: number
			columnDef: ColumnDef
			column: string
			value: any
			data: any
			oldValue: string | undefined
		}>
	) {
		const { columnDef, value, data, oldValue } = e.detail

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration[resolvedConfig.type.selected].resource,
			resolvedConfig.type.configuration[resolvedConfig.type.selected].table ?? 'unknown',
			columnDef,
			resolvedConfig.columnDefs,
			value,
			data,
			oldValue,
			resolvedConfig.type.selected as Preview.language
		)
	}

	let args: Record<string, any> = {}

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: {},
		selectedRows: [] as any[],
		result: [] as any[],
		loading: false,
		page: 0,
		newChange: { row: 0, column: '', value: undefined },
		ready: undefined as boolean | undefined
	})

	let lastResource: string | undefined = undefined

	function updateOneOfConfiguration<T, U extends string, V>(
		oneOfConfiguration: OneOfConfiguration,
		resolvedConfig: {
			configuration: Record<U, V>
			selected: U
		},
		patch: Partial<Record<keyof V, any>>
	) {
		const selectedConfig = oneOfConfiguration.configuration[resolvedConfig.selected]
		if (!selectedConfig) {
			console.warn(`Selected configuration '${resolvedConfig.selected}' does not exist.`)
			return
		}
		Object.keys(patch).forEach((key) => {
			oneOfConfiguration.configuration[resolvedConfig.selected][key] = {
				...oneOfConfiguration.configuration[resolvedConfig.selected][key],
				...patch[key]
			}
		})
	}

	async function listTableIfAvailable() {
		let resource = resolvedConfig.type.configuration?.[resolvedConfig.type.selected]?.resource
		if (lastResource === resource) return
		lastResource = resource
		const gridItem = findGridItem($app, id)

		if (!gridItem) {
			return
		}

		updateOneOfConfiguration(
			gridItem.data.configuration.type as OneOfConfiguration,
			resolvedConfig.type,
			{
				table: {
					selectOptions: [],
					loading: true
				}
			}
		)

		if (!resolvedConfig.type?.configuration?.[resolvedConfig.type.selected]?.resource) {
			$app = {
				...$app
			}
			return
		}

		try {
			const dbSchemas: DBSchemas = {}

			await getDbSchemas(
				resolvedConfig?.type?.selected,
				resolvedConfig.type.configuration[resolvedConfig?.type?.selected].resource.split(':')[1],
				$workspaceStore,
				dbSchemas,
				(message: string) => {}
			)

			updateOneOfConfiguration(
				gridItem.data.configuration.type as OneOfConfiguration,
				resolvedConfig.type,
				{
					table: {
						selectOptions: dbSchemas ? getTablesByResource(dbSchemas) : [],
						loading: false
					}
				}
			)

			$app = {
				...$app
			}
		} catch (e) {}
	}

	function getTablesByResource(schema: Partial<Record<string, DBSchema>>) {
		if (resolvedConfig.type.selected === 'postgresql') {
			// @ts-ignore
			return Object.keys(Object.values(schema)?.[0]?.schema?.public ?? {})
		} else if (resolvedConfig.type.selected === 'mysql') {
			return Object.keys(Object.values(Object.values(schema)?.[0]?.schema ?? {})?.[0])
		}
		return []
	}

	const cache = {
		params: {},
		data: [],
		promise: null // Store the promise of the ongoing request
	} as { data: any[]; params: Record<string, any>; promise: Promise<any> | null }

	function paramsChanged(currentParams: Record<string, any>) {
		if (Object.keys(cache.params).length === 0) return true
		return JSON.stringify(currentParams) !== JSON.stringify(cache.params)
	}

	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch,
				orderBy: params.sortModel?.[0]?.colId ?? resolvedConfig.columnDefs?.[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			}

			if (!paramsChanged(currentParams) && cache.data.length > 0) {
				// Serve from cache if it exists and parameters haven't changed.
				console.debug('Serving from cache for ID:', id)
				let lastRow = -1
				if (datasource.rowCount && datasource.rowCount <= params.endRow) {
					lastRow = datasource.rowCount
				}
				params.successCallback(cache.data, lastRow)
				return
			}

			// If parameters changed or no cache available, check for ongoing request
			if (!cache.promise || paramsChanged(currentParams)) {
				console.debug('Parameters changed or no ongoing request, fetching new data for ID:', id)
				cache.params = currentParams // Update the cache with the new parameters
				cache.promise = runnableComponent?.runComponent(
					undefined,
					undefined,
					undefined,
					currentParams,
					{
						done: (x) => {
							let lastRow = -1

							if (datasource.rowCount && datasource.rowCount <= params.endRow) {
								lastRow = datasource.rowCount
							}

							if (x && Array.isArray(x)) {
								let processedData = x.map((x) => {
									let primaryKeys = getPrimaryKeys(resolvedConfig.columnDefs)
									let o = {}
									primaryKeys.forEach((pk) => {
										o[pk] = x[pk]
									})
									x['__index'] = JSON.stringify(o)
									return x
								})

								cache.data = processedData // Update cache with new data
								params.successCallback(processedData, lastRow)
							} else {
								params.failCallback()
							}
							cache.promise = null
						},
						cancel: () => {
							params.failCallback()
							cache.promise = null
						},
						error: () => {
							params.failCallback()
							cache.promise = null
						}
					}
				)
			} else {
				console.debug('Request with same parameters already in progress, waiting for it to finish.')
				await cache.promise // Wait for the ongoing request to finish
				// After waiting, call getRows again to serve data from cache

				setTimeout(() => {
					this.getRows(params)
				}, 0)
			}
		}
	}

	let lastTable: string | undefined = undefined

	let timeout: NodeJS.Timeout | undefined = undefined
	async function listColumnsIfAvailable() {
		const selected = resolvedConfig.type.selected
		let table = resolvedConfig.type.configuration?.[selected]?.table
		if (lastTable === table) return
		lastTable = table

		let tableMetadata = await loadTableMetaData(
			resolvedConfig.type.configuration[selected].resource,
			$workspaceStore,
			resolvedConfig.type.configuration[selected].table,
			selected
		)

		if (!tableMetadata) return

		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		let columnDefs = gridItem.data.configuration.columnDefs as StaticInput<TableMetadata>
		let old: TableMetadata = (columnDefs?.value as TableMetadata) ?? []
		if (!Array.isArray(old)) {
			console.log('old is not an array RESET')
			old = []
		}
		// console.log('OLD', old)
		// console.log(tableMetadata)
		const oldMap = Object.fromEntries(old.filter((x) => x != undefined).map((x) => [x.field, x]))
		const newMap = Object.fromEntries(tableMetadata?.map((x) => [x.field, x]) ?? [])

		let ncols: any[] = []
		Object.entries(oldMap).forEach(([key, value]) => {
			if (newMap[key]) {
				ncols.push({
					...value,
					...newMap[key]
				})
			}
		})
		Object.entries(newMap).forEach(([key, value]) => {
			if (!oldMap[key]) {
				ncols.push(value)
			}
		})

		// Mysql capitalizes the column names, so we make sure to lowercase them
		ncols = ncols.map((x) => {
			let o = {}
			Object.keys(x).forEach((k) => {
				o[k.toLowerCase()] = x[k]
			})
			return o
		})

		state = undefined

		//@ts-ignore
		gridItem.data.configuration.columnDefs = { value: ncols, type: 'static' }
		gridItem.data = gridItem.data
		$app = $app
		let oldS = $selectedComponent
		$selectedComponent = []
		await tick()
		$selectedComponent = oldS

		//@ts-ignore
		resolvedConfig.columnDefs = ncols
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			timeout = undefined
			renderCount += 1
		}, 1500)
	}

	let isInsertable: boolean = false

	$: $worldStore && connectToComponents()

	function connectToComponents() {
		if ($worldStore && datasource) {
			const outputs = $worldStore.outputsById[`${id}_count`]
			if (outputs) {
				outputs.result.subscribe(
					{
						id: 'dbexplorer-count-' + id,
						next: (value) => {
							datasource.rowCount = value?.[0]?.count
						}
					},
					datasource.rowCount
				)
			}
		}
	}

	async function insert() {
		try {
			const selected = resolvedConfig.type.selected
			await insertRowRunnable?.insertRow(
				resolvedConfig.type.configuration[selected].resource,
				$workspaceStore,
				resolvedConfig.type.configuration[selected].table,
				resolvedConfig.columnDefs,
				args,
				selected
			)

			insertDrawer?.closeDrawer()
			renderCount++
		} catch (e) {
			sendUserToast(e.message, true)
		}

		args = {}
	}

	let runnableComponent: RunnableComponent
	let state: any = undefined
	let insertRowRunnable: InsertRowRunnable
	let deleteRow: DeleteRow

	function onDelete(e) {
		const data = { ...e.detail }
		delete data['__index']
		let primaryColumns = getPrimaryKeys(resolvedConfig.columnDefs)
		let getPrimaryKeysresolvedConfig = resolvedConfig.columnDefs?.filter((x) =>
			primaryColumns.includes(x.field)
		)
		const selected = resolvedConfig.type.selected
		deleteRow?.triggerDelete(
			resolvedConfig.type.configuration[selected].resource,
			resolvedConfig.type.configuration[selected].table ?? 'unknown',
			getPrimaryKeysresolvedConfig,
			data,
			selected as Preview.language
		)
	}

	let refreshCount = 0
</script>

{#each Object.keys(components['dbexplorercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		extraKey="db_explorer"
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InsertRowRunnable
	on:insert={() => {
		aggrid?.clearRows()
		refreshCount++
	}}
	{id}
	bind:this={insertRowRunnable}
/>
{#if resolvedConfig.allowDelete}
	<DeleteRow
		on:deleted={() => {
			aggrid?.clearRows()
			refreshCount++
		}}
		{id}
		bind:this={deleteRow}
	/>
{/if}
<UpdateCell {id} bind:this={updateCell} />
<DbExplorerCount
	renderCount={refreshCount}
	{id}
	{quicksearch}
	table={resolvedConfig?.type?.configuration?.[resolvedConfig?.type?.selected]?.table ?? ''}
	resource={resolvedConfig?.type?.configuration?.[resolvedConfig?.type?.selected]?.resource ?? ''}
	resourceType={resolvedConfig?.type?.selected}
	columnDefs={resolvedConfig?.columnDefs}
	whereClause={resolvedConfig?.whereClause}
/>

<InitializeComponent {id} />

<RunnableWrapper
	allowConcurentRequests
	noInitialize
	bind:runnableComponent
	componentInput={input}
	autoRefresh={false}
	{render}
	{id}
	{outputs}
>
	<div class="h-full" bind:clientHeight={componentContainerHeight}>
		<div class="flex p-2 justify-between gap-4" bind:clientHeight={buttonContainerHeight}>
			<input
				on:pointerdown|stopPropagation
				on:keydown|stopPropagation
				class="w-full max-w-[300px]"
				type="text"
				bind:value={quicksearch}
				placeholder="Quicksearch"
			/>
			<Button
				startIcon={{ icon: Plus }}
				color="dark"
				size="xs2"
				on:click={() => {
					args = {}
					insertDrawer?.openDrawer()
				}}
			>
				Insert
			</Button>
		</div>
		{#if resolvedConfig.type.configuration?.[resolvedConfig?.type?.selected]?.resource && resolvedConfig.type.configuration?.[resolvedConfig?.type?.selected]?.table}
			<!-- {JSON.stringify(lastInput)} -->
			<!-- <span class="text-xs">{JSON.stringify(configuration.columnDefs)}</span> -->
			{#key renderCount}
				<!-- {JSON.stringify(resolvedConfig.columnDefs)} -->
				<AppAggridExplorerTable
					bind:this={aggrid}
					bind:state
					{id}
					{datasource}
					{resolvedConfig}
					{customCss}
					{outputs}
					allowDelete={resolvedConfig.allowDelete ?? false}
					containerHeight={componentContainerHeight - buttonContainerHeight}
					on:update={onUpdate}
					on:delete={onDelete}
				/>
			{/key}
		{/if}
	</div>
</RunnableWrapper>
<Portal>
	<Drawer bind:this={insertDrawer} size="800px">
		<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button color="dark" size="xs" on:click={insert} disabled={!isInsertable}>Insert</Button>
			</svelte:fragment>

			<InsertRow
				bind:args
				bind:isInsertable
				columnDefs={resolvedConfig.columnDefs}
				databaseType={resolvedConfig.type.selected}
			/>
		</DrawerContent>
	</Drawer>
</Portal>

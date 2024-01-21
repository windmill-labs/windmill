<script lang="ts">
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { findGridItem, initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import {
		createPostgresInput,
		getDbSchemas,
		loadTableMetaData,
		type ColumnMetadata,
		type TableMetadata,
		getPrimaryKeys
	} from './utils'
	import { getContext, tick } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore, type DBSchemas } from '$lib/stores'
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
		resolvedConfig.type.configuration.postgresql.resource
	)

	function computeInput(columnDefs: any, whereClause: string | undefined, resource: any) {
		aggrid?.clearRows()
		input = createPostgresInput(
			resource,
			resolvedConfig.type.configuration.postgresql.table,
			columnDefs,
			whereClause
		)
	}

	$: editorContext != undefined && $mode == 'dnd' && resolvedConfig.type && listTableIfAvailable()

	$: editorContext != undefined &&
		$mode == 'dnd' &&
		resolvedConfig.type.configuration?.postgresql?.table &&
		listColumnsIfAvailable()

	$: if (quicksearch) {
		aggrid?.clearRows()
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
			columnDef: ColumnMetadata
			column: string
			value: any
			data: any
			oldValue: string | undefined
		}>
	) {
		const { columnDef, value, data, oldValue } = e.detail

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table ?? 'unknown',
			columnDef,
			resolvedConfig.columnDefs,
			value,
			data,
			oldValue
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
	async function listTableIfAvailable() {
		let resource = resolvedConfig.type.configuration?.postgresql?.resource
		if (lastResource === resource) return
		lastResource = resource
		const gridItem = findGridItem($app, id)

		if (!gridItem) {
			return
		}

		if (
			'configuration' in gridItem.data?.configuration?.type &&
			'selectOptions' in gridItem.data?.configuration?.type?.configuration?.postgresql?.table
		) {
			gridItem.data.configuration.type.configuration.postgresql.table.selectOptions = []
		}

		if (!resolvedConfig.type?.configuration?.postgresql?.resource) {
			$app = {
				...$app
			}
			return
		}

		if (
			'configuration' in gridItem.data?.configuration?.type &&
			gridItem.data.configuration.type.configuration.postgresql.table
		) {
			gridItem.data.configuration.type.configuration.postgresql.table.loading = true
		}

		try {
			const dbSchemas: DBSchemas = {}

			await getDbSchemas(
				'postgresql',
				resolvedConfig.type.configuration.postgresql.resource.split(':')[1],
				$workspaceStore,
				dbSchemas,
				(message: string) => {}
			)

			if ('configuration' in gridItem.data.configuration.type) {
				gridItem.data.configuration.type.configuration.postgresql.table['selectOptions'] = dbSchemas
					? // @ts-ignore
					  Object.keys(Object.values(dbSchemas)?.[0]?.schema?.public ?? {})
					: []
			}

			$app = {
				...$app
			}
		} catch (e) {}
		if (
			'configuration' in gridItem.data?.configuration?.type &&
			gridItem.data.configuration.type.configuration.postgresql.table
		) {
			gridItem.data.configuration.type.configuration.postgresql.table.loading = false
		}
	}

	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			refreshCount++
			let uuid = await runnableComponent?.runComponent(
				undefined,
				undefined,
				undefined,
				{
					offset: params.startRow,
					limit: params.endRow - params.startRow,
					quicksearch,
					orderBy: params.sortModel?.[0]?.colId ?? resolvedConfig.columnDefs?.[0]?.field,
					is_desc: params.sortModel?.[0]?.sort === 'desc'
				},
				{
					done: (x) => {
						if (x && Array.isArray(x)) {
							params.successCallback(
								x.map((x) => {
									let primaryKeys = getPrimaryKeys(resolvedConfig.columnDefs)
									let o = {}
									primaryKeys.forEach((pk) => {
										o[pk] = x[pk]
									})
									x['__index'] = JSON.stringify(o)
									return x
								}),
								datasource.rowCount
							)
						} else {
							params.failCallback()
						}
					},
					cancel: () => {
						console.log('cancel datasource request')
						params.failCallback()
					},
					error: () => {
						console.log('error datasource request')
						params.failCallback()
					}
				}
			)
			console.log('asking for ' + params.startRow + ' to ' + params.endRow, uuid)
		}
	}

	let lastTable: string | undefined = undefined
	async function listColumnsIfAvailable() {
		let table = resolvedConfig.type.configuration?.postgresql?.table
		if (lastTable === table) return
		lastTable = table

		let tableMetadata = await loadTableMetaData(
			resolvedConfig.type.configuration.postgresql.resource,
			$workspaceStore,
			resolvedConfig.type.configuration.postgresql.table
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
		renderCount += 1
	}

	let isInsertable: boolean = false

	$: $worldStore && connectToComponents()

	function connectToComponents() {
		if ($worldStore) {
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
			await insertRowRunnable?.insertRow(
				resolvedConfig.type.configuration.postgresql.resource,
				$workspaceStore,
				resolvedConfig.type.configuration.postgresql.table,
				resolvedConfig.columnDefs,
				args
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
		deleteRow?.triggerDelete(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table ?? 'unknown',
			getPrimaryKeysresolvedConfig,
			data
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
	table={resolvedConfig?.type?.configuration?.postgresql?.table ?? ''}
	resource={resolvedConfig?.type?.configuration?.postgresql?.resource ?? ''}
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
		{#if resolvedConfig.type.configuration?.postgresql?.resource && resolvedConfig.type.configuration?.postgresql?.table}
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

			<InsertRow bind:args bind:isInsertable columnDefs={resolvedConfig.columnDefs} />
		</DrawerContent>
	</Drawer>
</Portal>

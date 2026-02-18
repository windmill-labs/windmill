<script lang="ts">
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		OneOfConfiguration,
		RichConfigurations
	} from '../../../types'
	import { components, type TableAction } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { type TableMetadata, getPrimaryKeys, type ColumnDef } from './utils'
	import { getContext, tick, untrack } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore, type DBSchemas } from '$lib/stores'
	import { Drawer } from '$lib/components/common'

	import { sendUserToast } from '$lib/toast'
	import type { AppInput, StaticInput } from '$lib/components/apps/inputType'
	import DbExplorerCount from './DbExplorerCount.svelte'
	import AppAggridExplorerTable from '../table/AppAggridExplorerTable.svelte'
	import type { IDatasource } from 'ag-grid-community'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import InsertRowRunnable from './InsertRowRunnable.svelte'
	import DeleteRow from './DeleteRow.svelte'
	import InitializeComponent from '../../helpers/InitializeComponent.svelte'
	import { getSelectInput } from './queries/select'
	import DebouncedInput from '../../helpers/DebouncedInput.svelte'
	import { CancelablePromise } from '$lib/gen'
	import RefreshButton from '$lib/components/apps/components/helpers/RefreshButton.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import InsertRowDrawerButton from '../InsertRowDrawerButton.svelte'
	import { getDucklakeSchema } from '$lib/components/dbOps'
	import { findGridItem } from '$lib/components/apps/editor/appUtilsCore'
	import type { DbInput, DbType } from '$lib/components/dbTypes'
	import { getDbSchemas, getTablesByResource, loadTableMetaData } from './metadata'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'dbexplorercomponent'> | undefined
		render: boolean
		initializing?: boolean
		actions?: TableAction[]
	}

	let {
		id,
		configuration,
		customCss = undefined,
		render,
		initializing = $bindable(undefined),
		actions = []
	}: Props = $props()

	$effect.pre(() => {
		if (initializing === undefined) {
			initializing = true
		}
	})

	function clearColumns() {
		// We only want to clear the columns if the table has changed
		if (!(lastTable && table && lastTable !== table) && !(lastTable && !table)) {
			return
		}

		if (lastTable && !table) {
			lastTable = undefined
		}

		clearColumnDefs()
	}

	function clearColumnDefs() {
		const gridItem = findGridItem($app, id)

		if (!gridItem) {
			return
		}

		// @ts-ignore
		gridItem.data.configuration.columnDefs = { value: [], type: 'static', loading: false }

		$app = $app
	}

	const resolvedConfig = $state(
		initConfig(components['dbexplorercomponent'].initialData.configuration, configuration)
	)

	let timeoutInput: number | undefined = undefined

	function computeInput(columnDefs: any, whereClause: string | undefined, resource: any) {
		if (timeoutInput) {
			clearTimeout(timeoutInput)
		}
		timeoutInput = setTimeout(() => {
			timeoutInput = undefined
			console.log('compute input')

			input = getSelectInput(
				dbInput,
				resolvedConfig.type.configuration[resolvedConfig.type.selected].table,
				columnDefs,
				whereClause
			)
		}, 1000)
	}

	const { app, worldStore, mode, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')
	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	let input: AppInput | undefined = $state(undefined)
	let quicksearch = $state('')
	let aggrid: AppAggridExplorerTable | undefined = $state()

	let firstQuicksearch = $state(true)

	initializing = false

	let updateCell: UpdateCell | undefined = $state()

	let renderCount = $state(0)
	let insertDrawer: Drawer | undefined = undefined
	let componentContainerHeight: number | undefined = $state(undefined)
	let buttonContainerHeight: number | undefined = $state(undefined)

	let dbPath = $derived(
		resolvedConfig.type.selected === 'ducklake'
			? resolvedConfig.type.configuration?.[resolvedConfig.type.selected]?.ducklake
			: resolvedConfig.type.selected === 'datatable'
				? resolvedConfig.type.configuration?.[resolvedConfig.type.selected]?.datatable
				: resolvedConfig.type.configuration?.[resolvedConfig.type.selected]?.resource
	)
	let dbInput: DbInput = $derived(
		resolvedConfig.type.selected === 'ducklake'
			? {
					type: 'ducklake',
					ducklake: dbPath?.split('ducklake://')[1]
				}
			: {
					type: 'database',
					resourcePath: dbPath?.split('$res:')[1] ?? dbPath,
					resourceType:
						resolvedConfig.type.selected === 'datatable'
							? 'postgresql'
							: (resolvedConfig.type.selected as DbType)
				}
	)
	let dbtype = $derived(
		resolvedConfig.type.selected === 'ducklake'
			? ('duckdb' as const)
			: resolvedConfig.type.selected === 'datatable'
				? 'postgresql'
				: resolvedConfig.type.selected
	)

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
			dbInput,
			resolvedConfig.type.configuration[resolvedConfig.type.selected].table ?? 'unknown',
			columnDef,
			resolvedConfig.columnDefs,
			value,
			data,
			oldValue
		)
	}

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
		openedModalRow: {}
	})

	let lastResource: string | undefined = undefined

	function updateOneOfConfiguration<U extends string, V>(
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

	async function listTables() {
		if (!dbPath) return
		if (lastResource === dbPath) return
		lastResource = dbPath

		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		updateOneOfConfiguration(
			gridItem.data.configuration.type as OneOfConfiguration,
			resolvedConfig.type,
			{ table: { selectOptions: [], loading: true } }
		)

		if (!dbPath) {
			$app = $app
			return
		}

		try {
			const dbSchemas: DBSchemas = {}
			if (resolvedConfig?.type?.selected === 'ducklake') {
				dbSchemas[dbPath] = await getDucklakeSchema({
					workspace: $workspaceStore!,
					ducklake: dbPath?.split('ducklake://')[1]
				})
			} else {
				const resourcePath = dbPath?.split('$res:')[1] ?? dbPath
				dbSchemas[resourcePath] = await getDbSchemas(
					resolvedConfig?.type?.selected === 'datatable'
						? 'postgresql'
						: resolvedConfig?.type?.selected,
					resourcePath,
					$workspaceStore,
					() => {},
					{ useLegacyScripts: true }
				)
			}

			updateOneOfConfiguration(
				gridItem.data.configuration.type as OneOfConfiguration,
				resolvedConfig.type,
				{
					table: {
						selectOptions: dbSchemas
							? await getTablesByResource(dbSchemas, dbtype, dbPath, $workspaceStore!)
							: [],
						loading: false
					}
				}
			)

			$app = $app
		} catch (e) {}
	}

	let datasource: IDatasource = $state({
		rowCount: 0,
		getRows: async function (params) {
			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch,
				order_by: params.sortModel?.[0]?.colId ?? resolvedConfig.columnDefs?.[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			}

			if (!render) {
				return
			}

			if (!runnableComponent) {
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

						params.successCallback(processedData, lastRow)
					} else {
						params.failCallback()
					}
				},
				onCancel: () => {
					params.failCallback()
				},
				onError: (error) => {
					params.failCallback()
				}
			})
		}
	})

	let lastTable: string | undefined = $state(undefined)
	let timeout: number | undefined = undefined

	function isSubset(subset: Record<string, any>, superset: Record<string, any>) {
		return Object.keys(subset).every((key) => {
			return superset[key] === subset[key]
		})
	}

	function shouldReturnEarly(subset: Record<string, any>, superset: Record<string, any>): boolean {
		const subsetKeys = Object.keys(subset)
		const supersetKeys = Object.keys(superset)

		if (supersetKeys.length === 0) return false

		if (subsetKeys.length !== supersetKeys.length) {
			return false
		}

		if (
			JSON.stringify(supersetKeys.sort()) === JSON.stringify(subsetKeys.sort()) &&
			!subsetKeys.every((key) => isSubset(subset[key], superset[key]))
		) {
			return false
		}

		return true
	}

	async function listColumnsIfAvailable() {
		const selected = resolvedConfig.type.selected
		let table = resolvedConfig.type.configuration?.[resolvedConfig.type.selected]?.table

		if (lastTable === table) return

		lastTable = table

		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		let columnDefs = gridItem.data.configuration.columnDefs as StaticInput<TableMetadata>

		if (columnDefs.type !== 'static') return

		//@ts-ignore
		gridItem.data.configuration.columnDefs.loading = true
		gridItem.data = gridItem.data
		$app = $app

		let tableMetadata = await loadTableMetaData(
			resolvedConfig.type.selected === 'ducklake'
				? {
						type: 'ducklake',
						ducklake: dbPath?.split('ducklake://')[1]
					}
				: {
						type: 'database',
						resourcePath: dbPath?.split('$res:')[1] ?? dbPath,
						resourceType: dbtype
					},
			$workspaceStore,
			resolvedConfig.type.configuration[selected].table
		)

		if (!tableMetadata) return

		let old: TableMetadata = (columnDefs?.value as TableMetadata) ?? []
		if (!Array.isArray(old)) {
			console.log('old is not an array RESET')
			old = []
		}

		const oldMap = Object.fromEntries(old.filter((x) => x != undefined).map((x) => [x.field, x]))
		const newMap = Object.fromEntries(tableMetadata?.map((x) => [x.field, x]) ?? [])

		if (shouldReturnEarly(newMap, oldMap)) {
			//@ts-ignore
			gridItem.data.configuration.columnDefs.loading = false
			gridItem.data = gridItem.data

			$app = $app
			return
		}

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
				if (
					[
						'field',
						'datatype',
						'defaultvalue',
						'isprimarykey',
						'isidentity',
						'isnullable',
						'isenum'
					].includes(k.toLocaleLowerCase())
				) {
					o[k.toLowerCase()] = x[k]
				} else {
					o[k] = x[k]
				}
			})
			return o
		})

		componentState = undefined

		// If in the mean time the table has changed, we don't want to update the columnDefs
		if (lastTable !== table) {
			return
		}

		//@ts-ignore
		gridItem.data.configuration.columnDefs = { value: ncols, type: 'static', loading: false }
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

	function connectToComponents() {
		if ($worldStore && datasource !== undefined) {
			const outputs = $worldStore.outputsById[`${id}_count`]

			if (outputs) {
				outputs.result.subscribe(
					{
						id: 'dbexplorer-count-' + id,
						next: (value) => {
							if (value?.error) {
								const message = value?.error?.message ?? value?.error
								sendUserToast(message, true)
								return
							}

							// MsSql response have an outer array, we need to flatten it
							if (
								Array.isArray(value) &&
								value.length === 1 &&
								resolvedConfig.type.selected === 'ms_sql_server'
							) {
								// @ts-ignore
								datasource.rowCount = value?.[0]?.[0]?.count
							} else if (resolvedConfig.type.selected === 'snowflake') {
								// @ts-ignore
								datasource.rowCount = value?.[0]?.COUNT
							} else {
								// @ts-ignore
								datasource.rowCount = value?.[0]?.count
							}
						}
					},
					datasource.rowCount
				)
			}
		}
	}

	async function insert(args: object) {
		try {
			const selected = resolvedConfig.type.selected
			await insertRowRunnable?.insertRow(
				dbInput,
				$workspaceStore,
				resolvedConfig.type.configuration[selected].table,
				resolvedConfig.columnDefs,
				args
			)

			insertDrawer?.closeDrawer()
			renderCount++
		} catch (e) {
			sendUserToast(e.message, true)
		}
	}

	let runnableComponent: RunnableComponent | undefined = $state()
	let componentState: any = $state(undefined)
	let insertRowRunnable: InsertRowRunnable | undefined = $state()
	let deleteRow: DeleteRow | undefined = $state()
	let dbExplorerCount: DbExplorerCount | undefined = $state(undefined)

	function onDelete(e) {
		const data = { ...e.detail }
		delete data['__index']

		deleteRow?.triggerDelete(
			dbInput,
			resolvedConfig.type.configuration[resolvedConfig.type.selected].table ?? 'unknown',
			resolvedConfig.columnDefs,
			data
		)
	}

	let refreshCount = $state(0)

	let loading: boolean = $state(false)
	let table = $derived(
		resolvedConfig.type.configuration?.[resolvedConfig.type?.selected]?.table as string | undefined
	)
	$effect(() => {
		table !== null && render && untrack(() => clearColumns())
	})
	$effect(() => {
		;[resolvedConfig.columnDefs, resolvedConfig.whereClause, resolvedConfig.type.selected]
		resolvedConfig.type.selected &&
			render &&
			untrack(() => {
				computeInput(resolvedConfig.columnDefs, resolvedConfig.whereClause, dbPath)
			})
	})
	$effect(() => {
		editorContext != undefined &&
			$mode == 'dnd' &&
			resolvedConfig.type &&
			dbPath &&
			untrack(() => listTables())
	})
	$effect(() => {
		editorContext != undefined &&
			$mode == 'dnd' &&
			resolvedConfig.type.configuration?.[resolvedConfig?.type?.selected]?.table &&
			untrack(() => listColumnsIfAvailable())
	})
	$effect(() => {
		if (quicksearch !== undefined) {
			if (firstQuicksearch) {
				firstQuicksearch = false
			} else if (aggrid) {
				untrack(() => aggrid?.clearRows())
			}
		}
	})
	$effect(() => {
		$worldStore && render && untrack(() => connectToComponents())
	})
	let hideSearch = $derived(resolvedConfig.hideSearch as boolean)
	let hideInsert = $derived(resolvedConfig.hideInsert as boolean)
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
{#if render}
	<DbExplorerCount
		bind:this={dbExplorerCount}
		renderCount={refreshCount}
		{id}
		{quicksearch}
		{table}
		{dbInput}
		columnDefs={resolvedConfig?.columnDefs}
		whereClause={resolvedConfig?.whereClause}
	/>
{/if}

<InitializeComponent {id} />

<RunnableWrapper
	allowConcurentRequests
	noInitialize
	bind:runnableComponent
	componentInput={input}
	autoRefresh={false}
	bind:loading
	{render}
	{id}
	{outputs}
	overrideCallback={() =>
		new CancelablePromise(async (resolve) => {
			await dbExplorerCount?.computeCount(true)

			aggrid?.clearRows()
			resolve()
		})}
	overrideAutoRefresh={true}
>
	<div class="h-full" bind:clientHeight={componentContainerHeight}>
		{#if !(hideSearch === true && hideInsert === true)}
			<div class="flex py-2 h-12 justify-between gap-4" bind:clientHeight={buttonContainerHeight}>
				{#if hideSearch !== true}
					<DebouncedInput
						class="w-full max-w-[300px]"
						type="text"
						bind:value={quicksearch}
						placeholder="Search..."
					/>
				{/if}
				<div class="flex flex-row gap-2">
					<RefreshButton {id} {loading} />
					{#if hideInsert !== true}
						<InsertRowDrawerButton
							columnDefs={resolvedConfig.columnDefs}
							dbType={dbtype}
							onInsert={(args) => insert(args)}
						/>
					{/if}
				</div>
			</div>
		{/if}
		{#if dbPath && resolvedConfig.type.configuration?.[resolvedConfig?.type?.selected]?.table}
			<!-- {JSON.stringify(lastInput)} -->
			<!-- <span class="text-xs">{JSON.stringify(configuration.columnDefs)}</span> -->
			{#key renderCount && render}
				<!-- {JSON.stringify(resolvedConfig.columnDefs)} -->
				<AppAggridExplorerTable
					bind:this={aggrid}
					bind:componentState
					{id}
					{datasource}
					{resolvedConfig}
					{customCss}
					{outputs}
					allowDelete={resolvedConfig.allowDelete ?? false}
					containerHeight={componentContainerHeight - (buttonContainerHeight ?? 0)}
					on:update={onUpdate}
					on:delete={onDelete}
					allowColumnDefsActions={false}
					on:recompute={() => {
						lastTable = undefined
						clearColumnDefs()
						listColumnsIfAvailable()
					}}
					{actions}
				/>
			{/key}
		{/if}
	</div>
</RunnableWrapper>

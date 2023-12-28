<script context="module" lang="ts">
	export let tableMetadataShared: TableMetadata | undefined = undefined
</script>

<script lang="ts">
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { findGridItem, initConfig } from '$lib/components/apps/editor/appUtils'
	import {
		ColumnIdentity,
		createPostgresInput,
		getDbSchemas,
		insertRow,
		loadTableMetaData,
		type TableMetadata
	} from './utils'
	import AppAggridTable from '../table/AppAggridTable.svelte'
	import { getContext, onMount } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore, type DBSchemas } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import InsertRow from './InsertRow.svelte'
	import Portal from 'svelte-portal'
	import { sendUserToast } from '$lib/toast'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'dbexplorercomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let initializing: boolean = true

	const resolvedConfig = initConfig(
		components['dbexplorercomponent'].initialData.configuration,
		configuration
	)

	let mounted: boolean = false

	$: mounted && input && renderCount++

	onMount(() => {
		mounted = true
	})

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let tableMetaData: TableMetadata | undefined = undefined

	//$: hasOnePrimaryKey = tableMetaData?.filter((column) => column.isprimarykey).length === 1

	$: input = createPostgresInput(
		resolvedConfig.type.configuration.postgresql.resource,
		resolvedConfig.type.configuration.postgresql.table,
		resolvedConfig.columnDefs.filter((column) => column.ignored).map((column) => column.field),
		resolvedConfig.type.configuration.postgresql.pageSize,
		$worldStore.outputsById[id]?.page.peak() ?? 1,
		tableMetaData
	)

	async function toggleLoadTableData() {
		tableMetaData = await loadTableMetaData(
			resolvedConfig.type.configuration.postgresql.resource,
			$workspaceStore,
			resolvedConfig.type.configuration.postgresql.table
		)

		if (tableMetaData) {
			tableMetadataShared = tableMetaData
		}

		initializing = false
		renderCount++
	}

	let updateCell: UpdateCell
	let renderCount = 0
	let insertDrawer: Drawer | undefined = undefined
	let componentContainerHeight: number | undefined = undefined
	let buttonContainerHeight: number | undefined = undefined

	$: input && tableMetaData === undefined && toggleLoadTableData()

	function onUpdate(
		e: CustomEvent<{
			row: number
			column: string
			value: any
			data: any
			oldValue: string | undefined
		}>
	) {
		const { row, column, value, data, oldValue } = e.detail

		if (!tableMetaData) {
			sendUserToast('Table metadata is not available yet', true)
			return
		}

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table,
			row,
			column,
			value,
			data,
			tableMetaData,
			oldValue
		)
	}

	let args: Record<string, any> = {}

	async function listTableIfAvailable() {
		const gridItem = findGridItem($app, id)

		if (!gridItem) {
			return
		}

		if (!resolvedConfig.type.configuration.postgresql.resource) {
			if (
				'configuration' in gridItem.data.configuration.type &&
				'selectOptions' in gridItem.data.configuration.type.configuration.postgresql.table
			) {
				gridItem.data.configuration.type.configuration.postgresql.table.selectOptions = []
			}

			$app = {
				...$app
			}
			return
		}

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
	}

	$: resolvedConfig.type && listTableIfAvailable()

	function extractDefaultValue(defaultValue: string | undefined): string | undefined {
		if (defaultValue && defaultValue.includes('::')) {
			const val = defaultValue.split('::')[0]
			if (val.startsWith("'") && val.endsWith("'")) {
				return val.slice(1, -1)
			}
			return val
		}
		return defaultValue
	}

	let isInsertable: boolean = false

	async function insert() {
		try {
			const defaultValue = resolvedConfig.columnDefs.reduce((acc, column) => {
				const tableColumn = tableMetaData?.find((col) => col.columnname === column.field)
				const hasValue =
					args[column.field] !== undefined &&
					args[column.field] !== null &&
					args[column.field] !== ''
				const hasDefaultValue =
					column.defaultValue || extractDefaultValue(tableColumn?.defaultvalue)

				if (
					column.insert &&
					!tableColumn?.isnullable &&
					!hasValue &&
					!hasDefaultValue &&
					tableColumn?.isidentity !== ColumnIdentity.Always
				) {
					throw new Error(
						`Column ${column.field} requires a default value as it is non-nullable and no value is provided.`
					)
				}

				// Set the default value for columns with insert true, if no value is provided
				if (column.insert && !hasValue) {
					acc[column.field] = column.defaultValue || extractDefaultValue(tableColumn?.defaultvalue)
				}

				return acc
			}, {})

			const allArgs = { ...args, ...defaultValue }

			Object.keys(allArgs).forEach((key) => {
				if (allArgs[key] === null || allArgs[key] === undefined) {
					delete allArgs[key]
				}
			})

			await insertRow(
				resolvedConfig.type.configuration.postgresql.resource,
				$workspaceStore,
				resolvedConfig.type.configuration.postgresql.table,
				allArgs
			)

			insertDrawer?.closeDrawer()
			renderCount++
		} catch (e) {
			sendUserToast(e.message, true)
		}

		args = {}
	}
</script>

{#each Object.keys(components['dbexplorercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<UpdateCell {id} bind:this={updateCell} />
<div class="h-full" bind:clientHeight={componentContainerHeight}>
	<div class="flex flex-start justify-end p-2" bind:clientHeight={buttonContainerHeight}>
		<Button
			startIcon={{ icon: Plus }}
			color="dark"
			size="xs2"
			on:click={() => {
				insertDrawer?.openDrawer()
			}}
		>
			Insert
		</Button>
	</div>
	{#key renderCount}
		{#if resolvedConfig.type.configuration.postgresql.resource && resolvedConfig.type.configuration.postgresql.table}
			<AppAggridTable
				{id}
				{configuration}
				bind:initializing
				componentInput={input}
				{customCss}
				{render}
				containerHeight={componentContainerHeight - buttonContainerHeight}
				on:update={onUpdate}
			/>
		{/if}
	{/key}
</div>

<Portal>
	<Drawer bind:this={insertDrawer} size="800px">
		<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button color="dark" size="xs" on:click={insert} disabled={!tableMetaData || !isInsertable}>
					Insert
				</Button>
			</svelte:fragment>

			<InsertRow
				{tableMetaData}
				bind:args
				bind:isInsertable
				columnDefs={resolvedConfig.columnDefs}
			/>
		</DrawerContent>
	</Drawer>
</Portal>

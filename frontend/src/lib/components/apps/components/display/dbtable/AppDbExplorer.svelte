<script context="module" lang="ts">
	export let tableMetadataShared: TableMetadata | undefined = undefined
</script>

<script lang="ts">
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { findGridItem, initConfig } from '$lib/components/apps/editor/appUtils'
	import {
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
	}

	let updateCell: UpdateCell
	let renderCount = 0

	$: input && renderCount++
	$: tableMetaData === undefined && toggleLoadTableData()

	let insertDrawer: Drawer | undefined = undefined

	onMount(() => {
		toggleLoadTableData()
	})

	let componentContainerHeight: number | undefined = undefined
	let buttonContainerHeight: number | undefined = undefined

	function onUpdate(e: CustomEvent<{ row: number; column: string; value: any; data: any }>) {
		const { row, column, value, data } = e.detail

		if (!tableMetaData) {
			return
		}

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table,
			row,
			column,
			value,
			data,
			tableMetaData
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
	$: resolvedConfig.type.configuration.postgresql.resource && toggleLoadTableData()

	let isInsertable: boolean = false
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
	{/key}
</div>

<Portal>
	<Drawer bind:this={insertDrawer} size="800px">
		<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					color="dark"
					size="xs"
					on:click={async () => {
						await insertRow(
							resolvedConfig.type.configuration.postgresql.resource,
							$workspaceStore,
							resolvedConfig.type.configuration.postgresql.table,
							args
						)

						insertDrawer?.closeDrawer()
						renderCount++
					}}
					disabled={!tableMetaData || !isInsertable}
				>
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

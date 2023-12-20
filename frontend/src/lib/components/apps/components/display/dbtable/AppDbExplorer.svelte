<script lang="ts">
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { initConfig } from '$lib/components/apps/editor/appUtils'
	import { createPostgresInput, loadTableMetaData, type TableMetadata } from './utils'
	import AppAggridTable from '../table/AppAggridTable.svelte'
	import { getContext, onMount } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import InsertRow from './InsertRow.svelte'
	import Portal from 'svelte-portal'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	let initializing: boolean = true

	const resolvedConfig = initConfig(
		components['dbexplorercomponent'].initialData.configuration,
		configuration
	)

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	$: input = createPostgresInput(
		resolvedConfig.type.configuration.postgresql.resource,
		resolvedConfig.type.configuration.postgresql.table,
		resolvedConfig.type.configuration.postgresql.columns,
		resolvedConfig.type.configuration.postgresql.pageSize,
		$worldStore.outputsById[id]?.page.peak() ?? 1
	)

	let tableMetaData: TableMetadata | undefined = undefined

	async function toggleLoadTableData() {
		tableMetaData = await loadTableMetaData(
			resolvedConfig.type.configuration.postgresql.resource,
			$workspaceStore,
			resolvedConfig.type.configuration.postgresql.table
		)
		console.log(tableMetaData)
	}

	let updateCell: UpdateCell
	let renderCount = 0

	$: input && renderCount++
	$: tableMetaData === undefined && toggleLoadTableData()

	let insertDrawer: Drawer | undefined = undefined

	onMount(() => {
		toggleLoadTableData()
	})
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

<div class="flex flex-start p-2">
	<Button
		startIcon={{ icon: Plus }}
		color="dark"
		size="xs"
		on:click={() => {
			insertDrawer?.openDrawer()
		}}>Insert</Button
	>
</div>
{#key renderCount}
	<AppAggridTable
		{id}
		{configuration}
		bind:initializing
		componentInput={input}
		{customCss}
		{render}
		on:update={(e) => {
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
		}}
	/>
{/key}

<Portal>
	<Drawer bind:this={insertDrawer} size="800px">
		<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer}>
			<InsertRow
				{tableMetaData}
				on:insert={(e) => {
					const { data } = e.detail

					console.log(data)
				}}
			/>
		</DrawerContent>
	</Drawer>
</Portal>

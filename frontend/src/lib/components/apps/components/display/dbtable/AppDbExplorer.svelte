<script lang="ts">
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../../types'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { findGridItem, initConfig } from '$lib/components/apps/editor/appUtils'
	import {
		ColumnIdentity,
		createPostgresInput,
		getDbSchemas,
		insertRow,
		loadTableMetaData,
		type ColumnMetadata,
		type TableMetadata
	} from './utils'
	import AppAggridTable from '../table/AppAggridTable.svelte'
	import { getContext } from 'svelte'
	import UpdateCell from './UpdateCell.svelte'
	import { workspaceStore, type DBSchemas } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import InsertRow from './InsertRow.svelte'
	import Portal from 'svelte-portal'
	import { sendUserToast } from '$lib/toast'
	import type { StaticInput } from '$lib/components/apps/inputType'

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

	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')
	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	$: input = createPostgresInput(
		resolvedConfig.type.configuration.postgresql.resource,
		resolvedConfig.type.configuration.postgresql.table,
		resolvedConfig.columnDefs?.filter((column) => !column?.ignored) ?? [],
		resolvedConfig.pageSize,
		resolvedConfig.whereClause,
		$worldStore.outputsById[id]?.page.peak() ?? 1
	)

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
		const { row, columnDef, value, data, oldValue } = e.detail

		updateCell?.triggerUpdate(
			resolvedConfig.type.configuration.postgresql.resource,
			resolvedConfig.type.configuration.postgresql.table ?? 'unknown',
			row,
			columnDef,
			resolvedConfig.columnDefs,
			value,
			data,
			oldValue
		)
	}

	let args: Record<string, any> = {}

	$: input && resolvedConfig.pageSize && shouldReRender()

	let lastInput = input
	function shouldReRender() {
		if (lastInput !== input) {
			lastInput = input
			renderCount++
		}
	}

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

	$: editorContext != undefined && $mode == 'dnd' && resolvedConfig.type && listTableIfAvailable()

	$: editorContext != undefined &&
		$mode == 'dnd' &&
		resolvedConfig.type.configuration?.postgresql?.table &&
		listColumnsIfAvailable()

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
		const keys = tableMetadata.map((col) => col.field)
		Object.keys(old).forEach((key) => {
			if (!keys.includes(key)) {
				delete old[key]
			}
		})
		Object.entries(tableMetadata).forEach(([key, value], i) => {
			old[i] = {
				...old.find((col) => col?.field === key),
				...value
			}
		})

		//@ts-ignore
		gridItem.data.configuration.columnDefs = { value: old, type: 'static' }

		$app = $app
	}

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
				const hasValue =
					args[column.field] !== undefined &&
					args[column.field] !== null &&
					args[column.field] !== ''
				const hasDefaultValue = column.defaultValue || extractDefaultValue(column?.defaultvalue)

				if (
					column.insert &&
					!column?.isnullable &&
					!hasValue &&
					!hasDefaultValue &&
					column?.isidentity !== ColumnIdentity.Always
				) {
					throw new Error(
						`Column ${column.field} requires a default value as it is non-nullable and no value is provided.`
					)
				}

				// Set the default value for columns with insert true, if no value is provided
				if (column.insert && !hasValue) {
					acc[column.field] = column.defaultValue || extractDefaultValue(column?.defaultvalue)
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
		{#if resolvedConfig.type.configuration?.postgresql?.resource && resolvedConfig.type.configuration?.postgresql?.table}
			<!-- <span class="text-xs">{JSON.stringify(configuration.columnDefs)}</span> -->
			<AppAggridTable
				{id}
				{configuration}
				bind:initializing
				componentInput={input}
				{customCss}
				{render}
				pageSize={resolvedConfig.pageSize}
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
				<Button color="dark" size="xs" on:click={insert} disabled={!isInsertable}>Insert</Button>
			</svelte:fragment>

			<InsertRow bind:args bind:isInsertable columnDefs={resolvedConfig.columnDefs} />
		</DrawerContent>
	</Drawer>
</Portal>

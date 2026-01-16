<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import { sendUserToast, sortArray } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import { dbSupportsSchemas, type TableMetadata } from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import {
		dbSchemaOpsWithPreviewScripts,
		dbTableOpsWithPreviewScripts,
		getDbType,
		getDucklakeSchema
	} from './dbOps'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SqlRepl from './SqlRepl.svelte'
	import SimpleAgTable from './SimpleAgTable.svelte'
	import { untrack, type Snippet } from 'svelte'
	import type { DbInput } from './dbTypes'
	import {
		getDbSchemas,
		loadAllTablesMetaData,
		loadTableMetaData
	} from './apps/components/display/dbtable/metadata'

	import type { SelectedTable } from './DBManager.svelte'
	import { getDbFeatures } from './apps/components/display/dbtable/dbFeatures'

	interface Props {
		input?: DbInput
		showRepl?: boolean
		hasReplResult?: boolean
		isRefreshing?: boolean
		selectedSchemaKey?: string | undefined
		selectedTableKey?: string | undefined
		dbSelector?: Snippet<[]>
		/** Enable multi-select mode with checkboxes in sidebar */
		multiSelectMode?: boolean
		/** Selected tables in multi-select mode */
		selectedTables?: SelectedTable[]
		/** Tables that are already added and should show as disabled */
		disabledTables?: SelectedTable[]
	}

	let {
		input,
		showRepl = true,
		hasReplResult = $bindable(false),
		isRefreshing = $bindable(false),
		selectedSchemaKey = $bindable(undefined),
		selectedTableKey = $bindable(undefined),
		dbSelector,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = []
	}: Props = $props()

	let dbSchema: DBSchema | undefined = $derived(input && $dbSchemas[getDbSchemasPath(input)])

	function getDbSchemasPath(input: DbInput): string {
		switch (input.type) {
			case 'database':
				return input.resourcePath
			case 'ducklake':
				return 'ducklake://' + input.ducklake
		}
	}

	// `refreshCount` is a derived state. `refreshing` is the source of truth
	let refreshCount = $state(0)
	$effect(() => {
		if (refreshing) untrack(() => (refreshCount += 1))
	})

	let refreshing = $state(false)
	$effect(() => {
		if (refreshing) getSchema()
	})
	// Sync refreshing state with bindable prop
	$effect(() => {
		isRefreshing = refreshing
	})
	export const refresh = () => !refreshing && (refreshing = true)

	// Initial schema load
	$effect(() => {
		if (input) {
			untrack(() => getSchema())
		}
	})

	async function getSchema() {
		if (!input) return
		await Promise.all([
			(async () => {
				cachedColDefs = (await loadAllTablesMetaData($workspaceStore, input)) ?? cachedColDefs
			})(),
			(async () => {
				const dbSchemasPath = getDbSchemasPath(input)
				if ($dbSchemas[dbSchemasPath] && !refreshing) return

				const oldDbSchema = $dbSchemas[dbSchemasPath]
				if (input.type == 'database') {
					await getDbSchemas(
						input.resourceType,
						input.resourcePath,
						$workspaceStore,
						$dbSchemas,
						(message: string) => sendUserToast(message, true)
					)
				} else if (input.type == 'ducklake') {
					$dbSchemas[dbSchemasPath] = await getDucklakeSchema({
						workspace: $workspaceStore!,
						ducklake: input.ducklake
					})
				}
				// avoid infinite loop on error due to the way getDbSchemas is implemented
				// and relying on an assignement side effect
				if (oldDbSchema !== $dbSchemas[dbSchemasPath]) $dbSchemas = $dbSchemas
			})()
		])
		refreshing = false
	}

	let replPanelSize = $state(36)
	const REPL_MIN_SIZE = 1.5

	let replResultData: undefined | Record<string, any>[] = $state(undefined)

	// Sync replResultData state with bindable prop
	$effect(() => {
		hasReplResult = !!replResultData
	})

	let cachedColDefs: Record<string, TableMetadata> = {}
	let cachedLastRefreshCount = 0

	async function getColDefs(tableKey: string): Promise<TableMetadata> {
		if (cachedLastRefreshCount !== refreshCount) cachedColDefs = {}
		cachedLastRefreshCount = refreshCount

		if (cachedColDefs[tableKey]) return cachedColDefs[tableKey]
		if (!input) return []

		if (input?.type == 'ducklake') throw 'Impossible that loadAllTablesMetaData fails for Ducklake'
		// Query is not implemented for all dbs, need a fallback
		const result = await loadTableMetaData(input, $workspaceStore, tableKey)

		if (result) cachedColDefs[tableKey] = result
		return result ?? []
	}

	// Export for parent components
	export function clearReplResult() {
		replResultData = undefined
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			if (replResultData) {
				replResultData = undefined
			}
		}
	}}
/>

{#if dbSchema && $workspaceStore && input}
	{@const _input = input}
	{@const dbType = getDbType(_input)}
	<Splitpanes horizontal>
		<Pane class="relative">
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={'absolute inset-0 z-10 p-8 ' +
					(replResultData
						? 'bg-surface/90'
						: 'transition-colors bg-transparent pointer-events-none select-none')}
				onclick={(e) => {
					// Only proceed if the click is directly on this div and not on the child elements
					if (e.target === e.currentTarget) {
						replResultData = undefined
					}
				}}
			>
				{#if replResultData}
					{#key replResultData}
						<SimpleAgTable data={replResultData} class="animate-zoom-in" />
					{/key}
				{/if}
			</div>
			<DbManager
				dbSupportsSchemas={input?.type == 'database' && dbSupportsSchemas(input.resourceType)}
				{dbSchema}
				{getColDefs}
				dbTableOpsFactory={({ colDefs, tableKey }) =>
					dbTableOpsWithPreviewScripts({
						colDefs,
						tableKey,
						input: _input,
						workspace: $workspaceStore
					})}
				dbSchemaOps={dbSchemaOpsWithPreviewScripts({
					input: _input,
					workspace: $workspaceStore
				})}
				initialTableKey={input.specificTable}
				initialSchemaKey={input.type == 'database' ? input.specificSchema : undefined}
				{dbType}
				refresh={() => refresh()}
				{dbSelector}
				bind:selectedSchemaKey
				bind:selectedTableKey
				{multiSelectMode}
				bind:selectedTables
				{disabledTables}
				features={getDbFeatures(input)}
			/>
		</Pane>
		{#if showRepl}
			<Pane bind:size={replPanelSize} minSize={REPL_MIN_SIZE} class="relative">
				<SqlRepl
					{input}
					onData={(data) => {
						replResultData = data
					}}
					placeholderTableName={sortArray(
						Object.keys(
							dbSchema?.schema[
								'public' in dbSchema?.schema
									? 'public'
									: 'dbo' in dbSchema?.schema
										? 'dbo'
										: Object.keys(dbSchema?.schema ?? {})?.[0]
							] ?? {}
						)
					)?.[0]}
				/>
			</Pane>
		{/if}
	</Splitpanes>
{:else}
	<Splitpanes>
		<Pane class="relative flex justify-center items-center">
			<Loader2 class="animate-spin" size={32} />
		</Pane>
	</Splitpanes>
{/if}

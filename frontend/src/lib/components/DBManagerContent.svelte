<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import { sendUserToast, sortArray } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import { dbSupportsSchemas } from './apps/components/display/dbtable/utils'
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
	import { type Snippet } from 'svelte'
	import type { DbInput } from './dbTypes'
	import { getDbSchemas, loadAllTablesMetaData } from './apps/components/display/dbtable/metadata'

	import type { SelectedTable } from './DBManager.svelte'
	import { getDbFeatures } from './apps/components/display/dbtable/dbFeatures'
	import { resource } from 'runed'

	interface Props {
		input?: DbInput
		showRepl?: boolean
		hasReplResult?: boolean
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

	let colDefs = resource(
		() => [input],
		async () => {
			if (!input) return
			return await loadAllTablesMetaData($workspaceStore, input)
		}
	)
	let dbSchemasPromise = resource(
		() => [input],
		async () => {
			if (!input) return
			const dbSchemasPath = getDbSchemasPath(input)
			if (input.type == 'database') {
				$dbSchemas[dbSchemasPath] = await getDbSchemas(
					input.resourceType,
					input.resourcePath,
					$workspaceStore,
					(message: string) => sendUserToast(message, true)
				)
			} else if (input.type == 'ducklake') {
				$dbSchemas[dbSchemasPath] = await getDucklakeSchema({
					workspace: $workspaceStore!,
					ducklake: input.ducklake
				})
			}
		}
	)
	export const refresh = () => {
		colDefs.refetch()
		dbSchemasPromise.refetch()
	}
	export function isLoading() {
		return colDefs.loading || dbSchemasPromise.loading
	}

	let replPanelSize = $state(36)
	const REPL_MIN_SIZE = 1.5

	let replResultData: undefined | Record<string, any>[] = $state(undefined)

	// Sync replResultData state with bindable prop
	$effect(() => {
		hasReplResult = !!replResultData
	})

	// Export for parent components
	export function clearReplResult() {
		replResultData = undefined
	}
	let _dbManager: DbManager | undefined = $state()
	export const dbManager = () => _dbManager
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
				colDefs={colDefs.current}
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
				bind:this={_dbManager}
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

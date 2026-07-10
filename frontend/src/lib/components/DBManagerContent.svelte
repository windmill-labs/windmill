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
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from './common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { outOfOrderRunMessage } from './workspaceSettings/datatableMigrationUtils'

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
		onImport?: (mode: 'schema_and_data' | 'schema_only') => void
		/** Workspace the datatable/schema lookups run against. Defaults to the
		 *  navigation `$workspaceStore`; pass the acting workspace when embedded in
		 *  a session preview whose workspace differs from the top nav. */
		workspace?: string
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
		disabledTables = [],
		onImport,
		workspace = undefined
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let dbSchema: DBSchema | undefined = $derived(input && $dbSchemas[schemaCacheKey(input)])

	const outOfOrderModal = createAsyncConfirmationModal()

	function getDbSchemasPath(input: DbInput): string {
		switch (input.type) {
			case 'database':
				return input.resourcePath
			case 'ducklake':
				return 'ducklake://' + input.ducklake
		}
	}

	// Scope the shared `dbSchemas` cache by the acting workspace: a datatable of
	// the same name can exist in both the nav and the acting workspace, so the
	// bare resource path alone would let one workspace's schema be reused for the
	// other while DB operations target the acting one.
	function schemaCacheKey(input: DbInput): string {
		return `${ws}:${getDbSchemasPath(input)}`
	}

	let colDefs = resource(
		() => [input, ws],
		async () => {
			if (!input) return
			return await loadAllTablesMetaData(ws, input)
		}
	)
	let dbSchemasPromise = resource(
		() => [input, ws],
		async () => {
			if (!input) return
			const dbSchemasPath = schemaCacheKey(input)
			if (input.type == 'database') {
				$dbSchemas[dbSchemasPath] = await getDbSchemas(
					input.resourceType,
					input.resourcePath,
					ws,
					(message: string) => sendUserToast(message, true)
				)
			} else if (input.type == 'ducklake') {
				$dbSchemas[dbSchemasPath] = await getDucklakeSchema({
					workspace: ws!,
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

{#if dbSchema && ws && input}
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
				dbSupportsSchemas={dbSupportsSchemas(dbType)}
				databaseIsEmpty={!Object.values(dbSchema.schema).flatMap((s) => Object.values(s)).length}
				{dbSchema}
				colDefs={colDefs.current}
				dbTableOpsFactory={({ colDefs, tableKey }) =>
					dbTableOpsWithPreviewScripts({
						colDefs,
						tableKey,
						input: _input,
						workspace: ws
					})}
				dbSchemaOps={dbSchemaOpsWithPreviewScripts({
					input: _input,
					workspace: ws,
					confirmRunOutOfOrder: (pending) =>
						outOfOrderModal.ask({
							title: 'Run migration out of order',
							confirmationText: 'Run anyway',
							children: outOfOrderRunMessage(pending)
						})
				})}
				initialTableKey={input.specificTable}
				initialSchemaKey={input.specificSchema}
				asset={_input.type == 'ducklake'
					? { kind: 'ducklake', path: _input.ducklake }
					: _input.resourcePath.startsWith('datatable://')
						? { kind: 'datatable', path: _input.resourcePath.substring('datatable://'.length) }
						: undefined}
				{dbType}
				refresh={() => refresh()}
				{dbSelector}
				{onImport}
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
					{workspace}
					onData={(data) => {
						replResultData = data
					}}
					onSchemaChange={() => refresh()}
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

<Portal>
	<!-- Stacks above the DB table editor's own preview confirmation (z-[9999]),
		which is still open when applyDdl asks for out-of-order confirmation. -->
	<ConfirmationModal {...outOfOrderModal.props} zIndexClass="z-[10000]" />
</Portal>

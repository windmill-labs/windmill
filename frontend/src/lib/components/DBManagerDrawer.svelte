<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { sendUserToast, sortArray } from '$lib/utils'
	import { ArrowLeft, Expand, Loader2, Minimize, RefreshCcw } from 'lucide-svelte'
	import {
		dbSupportsSchemas,
		getDbSchemas,
		getLanguageByResourceType,
		loadAllTablesMetaData,
		loadTableMetaData,
		type TableMetadata
	} from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import {
		dbDeleteTableActionWithPreviewScript,
		dbTableOpsWithPreviewScripts,
		getDucklakeSchema,
		wrapDucklakeQuery,
		type DbInput
	} from './dbOps'
	import { makeCreateTableQuery } from './apps/components/display/dbtable/queries/createTable'
	import { runScriptAndPollResult } from './jobs/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SqlRepl from './SqlRepl.svelte'
	import SimpleAgTable from './SimpleAgTable.svelte'
	import { untrack } from 'svelte'

	let input: DbInput | undefined = $state()
	let open = $derived(!!input)

	export function openDrawer(nInput: DbInput) {
		input = nInput
		getSchema()
	}
	export function closeDrawer() {
		input = undefined
		refreshCount = 0
		refreshing = false
	}

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
	const refresh = () => !refreshing && (refreshing = true)

	let expand = $state(false)
	$effect(() => {
		if (!open) expand = false
	})

	async function getSchema() {
		if (!input) return
		const dbSchemasPath = getDbSchemasPath(input)
		if ($dbSchemas[dbSchemasPath] && !refreshing) return

		const oldDbSchema = $dbSchemas[dbSchemasPath]
		if (input.type == 'database') {
			await getDbSchemas(
				input.resourceType,
				input.resourcePath,
				$workspaceStore,
				$dbSchemas,
				(message: string) => {
					if (open) sendUserToast(message, true)
				}
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
		refreshing = false
	}

	let windowWidth = $state(window.innerWidth)

	let replPanelSize = $state(36)
	const REPL_MIN_SIZE = 1.5

	let replResultData: undefined | Record<string, any>[] = $state(undefined)

	let cachedColDefs: Record<string, TableMetadata> = {}
	let cachedLastRefreshCount = 0

	async function getColDefs(tableKey: string): Promise<TableMetadata> {
		if (cachedLastRefreshCount !== refreshCount) cachedColDefs = {}
		cachedLastRefreshCount = refreshCount

		if (cachedColDefs[tableKey]) return cachedColDefs[tableKey]
		if (!input) return []

		try {
			cachedColDefs = (await loadAllTablesMetaData($workspaceStore, input)) ?? cachedColDefs
			return cachedColDefs[tableKey]
		} catch (e) {
			if (input?.type == 'ducklake')
				throw 'Impossible that loadAllTablesMetaData fails for Ducklake'
			// Query is not implemented for all dbs, need a fallback
			const result = await loadTableMetaData(input, $workspaceStore, tableKey)

			if (result) cachedColDefs[tableKey] = result
			return result ?? []
		}
	}
</script>

<svelte:window
	bind:innerWidth={windowWidth}
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			if (replResultData) {
				replResultData = undefined
			}
		}
	}}
/>

<Drawer
	bind:open
	size={expand ? `${windowWidth}px` : '1200px'}
	preventEscape
	on:close={closeDrawer}
>
	{#key [input, dbSchema]}
		<DrawerContent
			title={replResultData ? 'Query Result' : 'Database Manager'}
			on:close={() => {
				if (replResultData) {
					replResultData = undefined
				} else {
					closeDrawer()
				}
			}}
			CloseIcon={replResultData ? ArrowLeft : undefined}
			noPadding
		>
			{#if dbSchema && $workspaceStore && input}
				{@const _input = input}
				{@const dbType = input.type == 'database' ? input.resourceType : 'duckdb'}
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
							dbSupportsSchemas={input.type == 'database' && dbSupportsSchemas(input.resourceType)}
							{dbSchema}
							{getColDefs}
							dbTableOpsFactory={({ colDefs, tableKey }) =>
								dbTableOpsWithPreviewScripts({
									colDefs,
									tableKey,
									input: _input,
									workspace: $workspaceStore
								})}
							dbTableActionsFactory={[
								dbDeleteTableActionWithPreviewScript({ input: _input, workspace: $workspaceStore })
							]}
							{refresh}
							dbTableEditorPropsFactory={({ selectedSchemaKey }) => ({
								dbType,
								previewSql: (values) => makeCreateTableQuery(values, dbType, selectedSchemaKey),
								async onConfirm(values) {
									const dbArg =
										input?.type === 'database' ? { database: '$res:' + input.resourcePath } : {}
									const language = getLanguageByResourceType(dbType)
									let query = makeCreateTableQuery(values, dbType, selectedSchemaKey)
									if (input?.type === 'ducklake') query = wrapDucklakeQuery(query, input.ducklake)
									await runScriptAndPollResult({
										workspace: $workspaceStore,
										requestBody: { args: dbArg, content: query, language }
									})
									refresh()
								}
							})}
						/>
					</Pane>
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
												: Object.keys(dbSchema?.schema)?.[0]
									]
								)
							)?.[0]}
						/>
					</Pane>
				</Splitpanes>
			{:else}
				<Splitpanes>
					<Pane class="relative flex justify-center items-center">
						<Loader2 class="animate-spin" size={32} />
					</Pane>
				</Splitpanes>
			{/if}
			{#snippet actions()}
				<Button
					loading={refreshing}
					on:click={() => refresh()}
					startIcon={{ icon: RefreshCcw }}
					size="xs"
					color="light"
				>
					Refresh
				</Button>

				<Button
					on:click={() => (expand = !expand)}
					startIcon={{ icon: expand ? Minimize : Expand }}
					size="xs"
					color="light"
				/>
			{/snippet}
		</DrawerContent>
	{/key}
</Drawer>

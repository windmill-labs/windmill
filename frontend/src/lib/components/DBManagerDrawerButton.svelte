<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { sendUserToast, sortArray } from '$lib/utils'
	import { ArrowLeft, Database, Expand, Loader2, Minimize, RefreshCcw } from 'lucide-svelte'
	import {
		dbSupportsSchemas,
		getDbSchemas,
		getLanguageByResourceType,
		loadAllTablesMetaData,
		loadTableMetaData,
		type DbType,
		type TableMetadata
	} from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import { Alert } from './common'
	import { dbDeleteTableActionWithPreviewScript, dbTableOpsWithPreviewScripts } from './dbOps'
	import { makeCreateTableQuery } from './apps/components/display/dbtable/queries/createTable'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import HideButton from './apps/editor/settingsPanel/HideButton.svelte'
	import SqlRepl from './SqlRepl.svelte'
	import SimpleAgTable from './SimpleAgTable.svelte'
	import { untrack } from 'svelte'

	type Props = {
		resourceType: DbType
		resourcePath: string
		class?: string
	}

	let { resourceType, resourcePath }: Props = $props()

	let dbSchema: DBSchema | undefined = $derived(
		resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
	)

	let isDrawerOpen: boolean = $state(false)

	let shouldDisplayError = $derived(
		resourcePath && resourcePath in $dbSchemas && !$dbSchemas[resourcePath]
	)

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
		if (!isDrawerOpen) expand = false
	})

	async function getSchema() {
		if ($dbSchemas[resourcePath] && !refreshing) return
		try {
			const oldDbSchema = $dbSchemas[resourcePath]
			await getDbSchemas(
				resourceType,
				resourcePath,
				$workspaceStore,
				$dbSchemas,
				(message: string) => {
					if (isDrawerOpen) {
						sendUserToast(message, true)
					}
				}
			)
			// avoid infinite loop on error due to the way getDbSchemas is implemented
			// and relying on an assignement side effect
			if (oldDbSchema !== $dbSchemas[resourcePath]) $dbSchemas = $dbSchemas
		} catch (e) {
			console.error(e)
		} finally {
			refreshing = false
		}
	}

	let windowWidth = $state(window.innerWidth)

	let replPanelSize = $state(36)
	const REPL_MIN_SIZE = 1.5

	let replResultData: undefined | Record<string, any>[] = $state(undefined)

	let cachedColDefs: Record<string, TableMetadata> = {}
	let cachedLastRefreshCount = 0

	async function getColDefs(tableKey: string) {
		if (cachedLastRefreshCount !== refreshCount) cachedColDefs = {}
		cachedLastRefreshCount = refreshCount
		if (cachedColDefs[tableKey]) {
			return cachedColDefs[tableKey]
		}
		try {
			cachedColDefs =
				(await loadAllTablesMetaData('$res:' + resourcePath, $workspaceStore, resourceType)) ??
				cachedColDefs
			return cachedColDefs[tableKey]
		} catch (e) {
			const result = await loadTableMetaData(
				'$res:' + resourcePath,
				$workspaceStore,
				tableKey,
				resourceType
			)

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

{#if shouldDisplayError}
	<Alert type="error" size="xs" title="Schema not available" class="mt-2">
		Schema could not be loaded. Please check the permissions of the resource.
	</Alert>
{:else}
	<Button
		size="xs"
		variant="border"
		spacingSize="xs2"
		btnClasses="mt-1 w-24"
		on:click={async () => {
			if (!dbSchema || !$workspaceStore) refreshing = true
			isDrawerOpen = true
		}}
	>
		<Database size={18} /> Manager
	</Button>
	<Drawer bind:open={isDrawerOpen} size={expand ? `${windowWidth}px` : '1200px'} preventEscape>
		<DrawerContent
			title={replResultData ? 'Query Result' : 'Database Manager'}
			on:close={() => {
				if (replResultData) {
					replResultData = undefined
				} else {
					isDrawerOpen = false
				}
			}}
			CloseIcon={replResultData ? ArrowLeft : undefined}
			noPadding
		>
			{#if dbSchema && $workspaceStore}
				<Splitpanes horizontal>
					<Pane class="relative">
						<div
							class={'absolute inset-0 z-10 p-12 ' +
								(replResultData
									? 'bg-surface/90'
									: 'transition-colors bg-transparent pointer-events-none select-none')}
						>
							{#if replResultData}
								<SimpleAgTable data={replResultData} class="animate-zoom-in" />
							{/if}
						</div>
						<DbManager
							dbSupportsSchemas={dbSupportsSchemas(resourceType)}
							{dbSchema}
							{getColDefs}
							dbTableOpsFactory={({ colDefs, tableKey }) =>
								dbTableOpsWithPreviewScripts({
									colDefs,
									tableKey,
									resourcePath,
									resourceType,
									workspace: $workspaceStore
								})}
							dbTableActionsFactory={[
								dbDeleteTableActionWithPreviewScript({
									resourcePath,
									resourceType,
									workspace: $workspaceStore
								})
							]}
							{refresh}
							dbTableEditorPropsFactory={({ selectedSchemaKey }) => ({
								resourceType,
								previewSql: (values) =>
									makeCreateTableQuery(values, resourceType, selectedSchemaKey),
								async onConfirm(values) {
									await runPreviewJobAndPollResult({
										workspace: $workspaceStore,
										requestBody: {
											args: { database: '$res:' + resourcePath },
											content: makeCreateTableQuery(values, resourceType, selectedSchemaKey),
											language: getLanguageByResourceType(resourceType)
										}
									})
									refresh()
								}
							})}
						/>
					</Pane>
					<Pane bind:size={replPanelSize} minSize={REPL_MIN_SIZE} class="relative">
						<div class="absolute top-2 right-6 z-10">
							<HideButton
								btnClasses="!text-primary border border-gray-200 dark:border-gray-600 bg-surface"
								direction="bottom"
								variant="contained"
								on:click={() => (replPanelSize = REPL_MIN_SIZE)}
							/>
						</div>
						<SqlRepl
							{resourcePath}
							{resourceType}
							onData={(data) => {
								replResultData = data
							}}
							placeholderTableName={sortArray(Object.keys(Object.values(dbSchema?.schema)[0]))[0]}
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
			<svelte:fragment slot="actions">
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
			</svelte:fragment>
		</DrawerContent>
	</Drawer>
{/if}

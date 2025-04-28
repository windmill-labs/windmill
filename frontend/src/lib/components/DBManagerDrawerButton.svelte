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
		loadTableMetaData,
		scripts,
		type DbType,
		type TableMetadata
	} from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import { Alert } from './common'
	import DbSchemaExplorer from './DBSchemaExplorer.svelte'
	import { dbDeleteTableActionWithPreviewScript, dbTableOpsWithPreviewScripts } from './dbOps'
	import { makeCreateTableQuery } from './apps/components/display/dbtable/queries/createTable'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { type ScriptLang } from '$lib/gen'
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

	console.log('dbSchemas', $dbSchemas)

	let isDrawerOpen: boolean = $state(false)
	let mode: 'db-manager' | 'schema-explorer' = $state('db-manager')

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

	$effect(() => {
		if (resourcePath && Object.keys(scripts).includes(resourceType)) {
			getSchema()
		}
	})

	let windowWidth = $state(window.innerWidth)

	let replPanelSize = $state(0)

	$effect(() => {
		if (mode !== 'db-manager') {
			replPanelSize = 0
			replResultData = undefined
		}
	})
	const openRepl = () => (replPanelSize = 32)
	openRepl()

	let replResultData: undefined | Record<string, any>[] = $state(undefined)

	let cachedColDefs: Record<string, TableMetadata> = {}
	let cachedLastRefreshCount = 0
	async function getColDefs(tableKey: string) {
		if (cachedLastRefreshCount !== refreshCount) cachedColDefs = {}
		cachedLastRefreshCount = refreshCount
		if (cachedColDefs[tableKey]) {
			return cachedColDefs[tableKey]
		}
		const result = await loadTableMetaData(
			'$res:' + resourcePath,
			$workspaceStore,
			tableKey,
			resourceType
		)
		if (result) cachedColDefs[tableKey] = result
		return result ?? []
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

{#if !dbSchema || !$workspaceStore}
	<Button size="xs" variant="border" spacingSize="xs2" btnClasses="mt-1 w-24" disabled>
		<Loader2 size={14} class="animate-spin" />
	</Button>
{:else if shouldDisplayError}
	<Alert type="error" size="xs" title="Schema not available" class="mt-2">
		Schema could not be loaded. Please check the permissions of the resource.
	</Alert>
{:else}
	<Button
		size="xs"
		variant="border"
		spacingSize="xs2"
		btnClasses="mt-1 w-24"
		on:click={() => (isDrawerOpen = true)}
	>
		<Database size={18} /> Manager
	</Button>
	<Drawer
		bind:open={isDrawerOpen}
		size={(
			{
				'db-manager': expand ? `${windowWidth}px` : '1200px',
				'schema-explorer': '500px'
			} satisfies Record<typeof mode, `${number}px`>
		)[mode]}
		preventEscape
	>
		<DrawerContent
			title={replResultData
				? 'Query Result'
				: (
						{
							'db-manager': 'Database Manager',
							'schema-explorer': 'Schema Explorer'
						} satisfies Record<typeof mode, string>
					)[mode]}
			on:close={() => {
				if (replResultData) {
					replResultData = undefined
				} else {
					isDrawerOpen = false
				}
			}}
			CloseIcon={replResultData ? ArrowLeft : undefined}
			noPadding={mode === 'db-manager'}
		>
			<Splitpanes horizontal>
				<Pane class="relative">
					<div
						class={'absolute inset-0 z-10 p-12 ' +
							(replResultData
								? 'bg-white/90'
								: 'transition-colors bg-transparent pointer-events-none select-none')}
					>
						{#if replResultData}
							<SimpleAgTable data={replResultData} class="animate-zoom-in" />
						{/if}
					</div>
					{#if mode === 'db-manager'}
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
							{refreshCount}
							dbTableEditorPropsFactory={({ selectedSchemaKey }) => ({
								resourceType,
								previewSql: (values) =>
									makeCreateTableQuery(values, resourceType, selectedSchemaKey),
								async onConfirm(values) {
									console.log(
										'onConfirm',
										makeCreateTableQuery(values, resourceType, selectedSchemaKey)
									)
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
					{:else if mode === 'schema-explorer'}
						<DbSchemaExplorer {dbSchema} />
					{/if}
				</Pane>
				<Pane bind:size={replPanelSize} class="relative">
					<div class="absolute top-2 right-6 z-10">
						<HideButton
							btnClasses="!text-primary border border-gray-200 dark:border-gray-600 bg-surface"
							direction="bottom"
							variant="contained"
							on:click={() => (replPanelSize = 0)}
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
			<svelte:fragment slot="actions">
				{#if !replPanelSize && mode === 'db-manager'}
					<Button
						btnClasses="!font-normal hover:text-primary text-primary/70"
						size="xs"
						color="light"
						on:click={openRepl}
					>
						SQL Repl
					</Button>
				{/if}
				<Button
					btnClasses="!font-normal hover:text-primary text-primary/70"
					size="xs"
					color="light"
					on:click={() => (mode = mode === 'db-manager' ? 'schema-explorer' : 'db-manager')}
				>
					{mode === 'db-manager' ? 'Explore schema' : 'Manage database'}
				</Button>

				<Button
					loading={refreshing}
					on:click={() => refresh()}
					startIcon={{ icon: RefreshCcw }}
					size="xs"
					color="light"
				>
					Refresh
				</Button>

				{#if mode === 'db-manager'}
					<Button
						on:click={() => (expand = !expand)}
						startIcon={{ icon: expand ? Minimize : Expand }}
						size="xs"
						color="light"
					/>
				{/if}
			</svelte:fragment>
		</DrawerContent>
	</Drawer>
{/if}

<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import { sortArray } from '$lib/utils'
	import { Loader2, RefreshCcw } from 'lucide-svelte'
	import Alert from './common/alert/Alert.svelte'
	import Button from './common/button/Button.svelte'
	import { dbSupportsSchemas } from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import DbWorkerTagPicker from './DbWorkerTagPicker.svelte'
	import MissingWorkerTagAlert from './jobs/MissingWorkerTagAlert.svelte'
	import {
		dbSchemaOpsWithPreviewScripts,
		dbTableOpsWithPreviewScripts,
		getDbType,
		getDefaultDbTag,
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
		/** Worker tag every job of this manager runs on, overriding the database
		 *  language's native tag. Bound so the hints below can offer to set it. */
		workerTag?: string
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
		workspace = undefined,
		workerTag = $bindable()
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

	// Reported in place of the loading spinner: both queries run as jobs, so
	// anything from a bad connection to a tag no worker serves surfaces here
	// instead of leaving the manager spinning with no explanation. Each query
	// owns its slot so neither can clear the other's error on a refetch.
	let schemaError = $state<string | undefined>(undefined)
	let colDefsError = $state<string | undefined>(undefined)
	let loadError = $derived(
		schemaError
			? { title: 'Could not load the database schema', message: schemaError }
			: colDefsError
				? { title: 'Could not load the tables of this database', message: colDefsError }
				: undefined
	)

	// A query already handed to the queue cannot be taken back: `resource` aborts its
	// controller, but the poller behind these queries doesn't watch the signal, and a
	// job left on a tag no worker serves only fails ~90s later. Stamp each run so a
	// superseded one can neither report its failure nor overwrite what replaced it —
	// picking a working tag from the hints below is exactly that race.
	let colDefsRun = 0
	let schemaRun = 0

	let colDefs = resource(
		() => [input, ws, workerTag],
		async () => {
			const run = ++colDefsRun
			colDefsError = undefined
			if (!input) return
			try {
				const metadata = await loadAllTablesMetaData(ws, input, workerTag)
				return run === colDefsRun ? metadata : colDefs.current
			} catch (e) {
				if (run !== colDefsRun) return colDefs.current
				colDefsError = 'Error loading tables metadata: ' + ((e as Error)?.message || e)
				return
			}
		}
	)

	let dbSchemasPromise = resource(
		() => [input, ws, workerTag],
		async () => {
			const run = ++schemaRun
			schemaError = undefined
			if (!input) return
			const dbSchemasPath = schemaCacheKey(input)
			if (input.type == 'database') {
				// Reported through a local, not `schemaError` directly, so a superseded
				// run's callback can't fail a load that already succeeded.
				let queryError: string | undefined
				const schema = await getDbSchemas(
					input.resourceType,
					input.resourcePath,
					ws,
					(message: string) => (queryError = message),
					{ customTag: workerTag }
				)
				if (run !== schemaRun) return
				if (!schema) {
					schemaError = queryError ?? 'The schema query returned no schema'
					return
				}
				$dbSchemas[dbSchemasPath] = schema
			} else if (input.type == 'ducklake') {
				try {
					const schema = await getDucklakeSchema({
						workspace: ws!,
						ducklake: input.ducklake,
						tag: workerTag
					})
					if (run === schemaRun) $dbSchemas[dbSchemasPath] = schema
				} catch (e) {
					if (run !== schemaRun) return
					schemaError = 'Error fetching schema: ' + ((e as Error)?.message || e)
				}
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

	// Knowing the tag up front is what makes the missing-worker hint below possible
	// without waiting for the poller to give up.
	let defaultTag = $derived(input ? getDefaultDbTag(input) : undefined)
	let jobTag = $derived(workerTag ?? defaultTag)

	// A job queued behind busy workers still loads eventually, so this is a hint
	// rather than an error. The no-worker-at-all case fails outright instead.
	const SLOW_LOAD_MS = 10_000
	let slowLoad = $state(false)
	$effect(() => {
		if (!isLoading()) {
			slowLoad = false
			return
		}
		const t = setTimeout(() => (slowLoad = true), SLOW_LOAD_MS)
		return () => clearTimeout(t)
	})

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

<!-- The error branch comes first on purpose: `dbSchema` is read from a cache that
	survives a failed refetch, so ordering it first would hide the failure behind
	stale content. -->
{#if loadError}
	<div class="h-full w-full flex flex-col items-center justify-center gap-3 p-8">
		<div class="max-w-2xl w-full flex flex-col gap-3">
			<Alert type="error" title={loadError.title} size="xs">
				{loadError.message}
			</Alert>
			<div class="self-start">
				<Button size="xs" color="light" startIcon={{ icon: RefreshCcw }} on:click={() => refresh()}>
					Retry
				</Button>
			</div>
			<!-- A database that no default worker can reach fails here rather than in the
				missing-worker path below, so the same override is offered on any load error. -->
			<DbWorkerTagPicker
				bind:tag={workerTag}
				{defaultTag}
				workspace={ws}
				class="border-t pt-3 max-w-md"
			/>
		</div>
	</div>
{:else if dbSchema && ws && input}
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
						workspace: ws,
						tag: workerTag
					})}
				dbSchemaOps={dbSchemaOpsWithPreviewScripts({
					input: _input,
					workspace: ws,
					tag: workerTag,
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
					tag={workerTag}
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
		<Pane class="relative flex flex-col justify-center items-center gap-3 p-8">
			<Loader2 class="animate-spin" size={32} />
			{#if slowLoad}
				<span class="text-xs text-tertiary max-w-md text-center">
					The schema query is taking a while. It runs as a job, so it waits for a worker serving its
					tag to be free.
				</span>
				{#if jobTag}
					<MissingWorkerTagAlert
						tag={jobTag}
						subject="Database queries"
						workspace={ws}
						class="max-w-2xl w-full"
					/>
				{/if}
				<DbWorkerTagPicker
					bind:tag={workerTag}
					{defaultTag}
					workspace={ws}
					class="max-w-md w-full"
				/>
			{/if}
		</Pane>
	</Splitpanes>
{/if}

<Portal>
	<!-- Stacks above the DB table editor's own preview confirmation (z-[9999]),
		which is still open when applyDdl asks for out-of-order confirmation. -->
	<ConfirmationModal {...outOfOrderModal.props} zIndexClass="z-[10000]" />
</Portal>

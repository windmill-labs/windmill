<script lang="ts">
	import {
		apiSchemaToEditorSchema,
		computeDatatableDiff,
		generateMigrationSql,
		type DatatableDiff,
		type TableDiff
	} from './datatableSchemaSql'
	import { WorkspaceService } from '$lib/gen'
	import { Loader2, ChevronDown, ChevronRight, Plus, Minus, Pencil, Eye } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { sendUserToast } from '$lib/toast'
	import { userWorkspaces } from '$lib/stores'
	import { runScriptAndPollResult } from '$lib/components/jobs/utils'
	import YAML from 'yaml'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from './common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import {
		pendingMigrations,
		outOfOrderRunMessage
	} from './workspaceSettings/datatableMigrationUtils'
	import Alert from './common/alert/Alert.svelte'
	import ResizeTransitionWrapper from './common/ResizeTransitionWrapper.svelte'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
	}

	let { currentWorkspaceId, parentWorkspaceId }: Props = $props()

	let loading = $state(true)
	let error: string | undefined = $state(undefined)
	let diffs: DatatableDiff[] = $state([])
	// Number of forked datatables this schema-diff section applies to: those that
	// have NOT opted in to the migrations feature. When a datatable enables
	// migrations, its changes flow through the normal item diff instead, so it is
	// excluded here and the whole section hides once none remain.
	let applicableCount = $state(0)
	let expandedDatatables: Set<string> = $state(new Set())

	// Drawer state
	let drawerOpen = $state(false)
	let drawerChange: TableDiff | undefined = $state(undefined)
	let drawerDiff: DatatableDiff | undefined = $state(undefined)
	let drawerDirection: 'ahead' | 'behind' | undefined = $state(undefined)
	let migrationSql = $state('')
	let migrationRunning = $state(false)
	let confirmDeployOpen = $state(false)
	const outOfOrderModal = createAsyncConfirmationModal()

	async function loadDiffs() {
		loading = true
		error = undefined
		diffs = []
		try {
			const forkSettings = await WorkspaceService.getPublicSettings({
				workspace: currentWorkspaceId
			})
			const datatables = forkSettings.datatable?.datatables ?? {}
			const forkedEntries = Object.entries(datatables).filter(
				([_, dt]) => dt.forked_from != null && dt.migrations_enabled !== true
			)
			applicableCount = forkedEntries.length
			if (forkedEntries.length === 0) {
				loading = false
				return
			}
			const results: DatatableDiff[] = []
			for (const [dtName, dt] of forkedEntries) {
				try {
					const originalSchema = apiSchemaToEditorSchema((dt.forked_from as any)?.schema ?? {})
					const [parentSchemaRaw, forkSchemaRaw] = await Promise.all([
						WorkspaceService.getDatatableFullSchema({
							workspace: parentWorkspaceId,
							requestBody: { source: `datatable://${dtName}` }
						}),
						WorkspaceService.getDatatableFullSchema({
							workspace: currentWorkspaceId,
							requestBody: { source: `datatable://${dtName}` }
						})
					])
					const parentSchema = apiSchemaToEditorSchema(parentSchemaRaw)
					const forkSchema = apiSchemaToEditorSchema(forkSchemaRaw)
					const diff = computeDatatableDiff(dtName, originalSchema, parentSchema, forkSchema)
					if (diff.aheadChanges.length > 0 || diff.behindChanges.length > 0) {
						results.push(diff)
					}
				} catch (e: any) {
					console.error(`Failed to diff datatable ${dtName}:`, e)
				}
			}
			diffs = results
		} catch (e: any) {
			error = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		void [currentWorkspaceId, parentWorkspaceId]
		loadDiffs()
	})

	function toggleExpanded(name: string) {
		const next = new Set(expandedDatatables)
		if (next.has(name)) next.delete(name)
		else next.add(name)
		expandedDatatables = next
	}

	function operationSummary(d: TableDiff): string {
		if (d.kind === 'added') return 'New table'
		if (d.kind === 'removed') return 'Deleted table'
		const ops = d.operations?.operations ?? []
		const parts: string[] = []
		const adds = ops.filter((o) => o.kind === 'addColumn').length
		const drops = ops.filter((o) => o.kind === 'dropColumn').length
		const alters = ops.filter((o) => o.kind === 'alterColumn').length
		const renames = ops.filter((o) => o.kind === 'renameTable').length
		const fkAdds = ops.filter((o) => o.kind === 'addForeignKey').length
		const fkDrops = ops.filter((o) => o.kind === 'dropForeignKey').length
		const pkChanges = ops.filter(
			(o) => o.kind === 'addPrimaryKey' || o.kind === 'dropPrimaryKey'
		).length
		if (adds) parts.push(`+${adds} col`)
		if (drops) parts.push(`-${drops} col`)
		if (alters) parts.push(`~${alters} col`)
		if (renames) parts.push('renamed')
		if (fkAdds) parts.push(`+${fkAdds} FK`)
		if (fkDrops) parts.push(`-${fkDrops} FK`)
		if (pkChanges) parts.push('PK changed')
		return parts.join(', ') || 'Modified'
	}

	function openReview(change: TableDiff, diff: DatatableDiff, direction: 'ahead' | 'behind') {
		drawerChange = change
		drawerDiff = diff
		drawerDirection = direction
		// ahead = fork changed → migration runs on parent to deploy
		// behind = parent changed → migration runs on fork to update
		const sourceSchema = direction === 'ahead' ? diff.forkSchema : diff.parentSchema
		migrationSql =
			'-- Migration is auto-generated on a best-effort basis. You can adjust it here \n\n' +
			generateMigrationSql(change, sourceSchema)
		drawerOpen = true
	}

	function getDiffYaml(): { original: string; modified: string } {
		if (!drawerChange || !drawerDiff || !drawerDirection) return { original: '', modified: '' }
		const { schemaName, tableName } = drawerChange
		const origTable = drawerDiff.originalSchema[schemaName]?.[tableName]
		// ahead = fork changed → show original vs fork
		// behind = parent changed → show original vs parent
		const changedSchema =
			drawerDirection === 'ahead' ? drawerDiff.forkSchema : drawerDiff.parentSchema
		const changedTable = changedSchema[schemaName]?.[tableName]
		return {
			original: origTable ? YAML.stringify(origTable) : '# table does not exist',
			modified: changedTable ? YAML.stringify(changedTable) : '# table does not exist'
		}
	}

	async function runMigration() {
		if (!drawerDiff || !drawerChange || !drawerDirection) return
		migrationRunning = true

		// ahead → run on parent; behind → run on fork
		const targetWorkspace = drawerDirection === 'ahead' ? parentWorkspaceId : currentWorkspaceId
		const dtName = drawerDiff.datatableName

		try {
			// If the target data table opted in to migrations, record this merge as a
			// tracked migration (named after the fork) and run it, instead of applying
			// raw SQL that would bypass the target's migration history.
			// Don't swallow a status-check failure by defaulting to raw apply: that
			// would apply the DDL untracked (schema drift) — exactly what this feature
			// prevents. Let the error propagate (fail closed, handled by the outer
			// catch); only fall back to raw apply when the API explicitly returns
			// enabled === false.
			const status = await WorkspaceService.getDatatableMigrationsStatus({
				workspace: targetWorkspace,
				datatableName: dtName
			})

			if (status.enabled) {
				// The merge migration gets the highest timestamp, so any still-pending
				// migration on the target is earlier: running only the merge applies it
				// out of order. Warn like the row-level Run action does.
				const pending = pendingMigrations(status.migrations)
				if (pending.length > 0) {
					const confirmed = await outOfOrderModal.ask({
						title: 'Run migration out of order',
						confirmationText: 'Run anyway',
						children: outOfOrderRunMessage(pending.length)
					})
					if (!confirmed) {
						migrationRunning = false
						return
					}
				}
				const forkName =
					$userWorkspaces.find((w) => w.id === currentWorkspaceId)?.name || currentWorkspaceId
				const migName = `merge_${forkName}`.replace(/[^a-zA-Z0-9_-]+/g, '_')
				const created = await WorkspaceService.createDatatableMigration({
					workspace: targetWorkspace,
					datatableName: dtName,
					requestBody: { name: migName, code_up: migrationSql }
				})
				try {
					await WorkspaceService.runDatatableMigrations({
						workspace: targetWorkspace,
						datatableName: dtName,
						only: created.timestamp
					})
				} catch (runErr: any) {
					// Undo the insertion so the user can fix and retry from a clean state.
					await WorkspaceService.deleteDatatableMigration({
						workspace: targetWorkspace,
						datatableName: dtName,
						timestamp: created.timestamp
					}).catch(() => {})
					throw runErr
				}
			} else {
				await runScriptAndPollResult({
					workspace: targetWorkspace,
					requestBody: {
						args: { database: `datatable://${dtName}` },
						language: 'postgresql',
						content: migrationSql
					}
				})
			}
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
			migrationRunning = false
			return
		}

		// Update forked_from.schema for the migrated table
		try {
			const sourceSchema =
				drawerDirection === 'ahead' ? drawerDiff.forkSchema : drawerDiff.parentSchema
			const { schemaName, tableName } = drawerChange
			const newTableDef = sourceSchema[schemaName]?.[tableName]

			const forkSettings = await WorkspaceService.getPublicSettings({
				workspace: currentWorkspaceId
			})
			const datatableConfig = forkSettings.datatable ?? { datatables: {} }
			const dtConfig = datatableConfig.datatables[dtName]
			if (dtConfig?.forked_from) {
				const forkedFrom = dtConfig.forked_from as any
				if (!forkedFrom.schema) forkedFrom.schema = {}
				if (!forkedFrom.schema[schemaName]) forkedFrom.schema[schemaName] = {}
				if (newTableDef) {
					forkedFrom.schema[schemaName][tableName] = newTableDef
				} else {
					delete forkedFrom.schema[schemaName][tableName]
				}
				await WorkspaceService.editDataTableConfig({
					workspace: currentWorkspaceId,
					requestBody: { settings: datatableConfig }
				})
			}
		} catch (e: any) {
			console.error('Failed to update forked_from schema:', e)
		}

		migrationRunning = false
		drawerOpen = false
		sendUserToast('Migration applied successfully')
		await loadDiffs()
	}
</script>

{#if applicableCount > 0}
	<div class="bg-surface-tertiary p-4 rounded-md border">
		<h3 class="text-sm font-semibold">Datatable schema changes</h3>
		{#if loading}
			<div class="flex items-center gap-2 text-xs text-tertiary py-2">
				<Loader2 class="w-4 h-4 animate-spin" /> Loading datatable diffs...
			</div>
		{:else if error}
			<div class="text-xs text-red-500 py-2">Failed to load datatable diffs: {error}</div>
		{:else if diffs.length > 0}
			<div class="flex flex-col gap-2 mt-3 mb-1">
				{#each diffs as diff}
					<ResizeTransitionWrapper class="border rounded-md" innerClass="w-full" vertical>
						<button
							class="w-full flex items-center justify-between px-3 py-2 hover:bg-surface-hover"
							onclick={() => toggleExpanded(diff.datatableName)}
						>
							<span class="text-xs font-medium">{diff.datatableName}</span>
							<div class="flex items-center gap-2 text-2xs text-tertiary">
								{#if diff.aheadChanges.length > 0}
									<span class="text-blue-500">{diff.aheadChanges.length} ahead</span>
								{/if}
								{#if diff.behindChanges.length > 0}
									<span class="text-orange-500">{diff.behindChanges.length} behind</span>
								{/if}
								{#if expandedDatatables.has(diff.datatableName)}
									<ChevronDown class="w-3 h-3" />
								{:else}
									<ChevronRight class="w-3 h-3" />
								{/if}
							</div>
						</button>

						{#if expandedDatatables.has(diff.datatableName)}
							<div class="border-t divide-y">
								{#if diff.aheadChanges.length > 0}
									<div class="px-3 py-1.5">
										<div class="text-2xs font-semibold text-blue-500 mb-1">Fork changes (ahead)</div
										>
										{#each diff.aheadChanges as change}
											<div class="flex items-center gap-2 text-xs py-0.5">
												{#if change.kind === 'added'}
													<Plus class="w-3 h-3 text-green-500 shrink-0" />
												{:else if change.kind === 'removed'}
													<Minus class="w-3 h-3 text-red-500 shrink-0" />
												{:else}
													<Pencil class="w-3 h-3 text-yellow-500 shrink-0" />
												{/if}
												<span class="text-tertiary">{change.schemaName}.</span>
												<span class="font-medium">{change.tableName}</span>
												<span class="text-tertiary text-2xs grow">{operationSummary(change)}</span>
												<Button
													size="xs"
													variant="subtle"
													startIcon={{ icon: Eye }}
													onclick={() => openReview(change, diff, 'ahead')}
												>
													Review
												</Button>
											</div>
										{/each}
									</div>
								{/if}
								{#if diff.behindChanges.length > 0}
									<div class="px-3 py-1.5">
										<div class="text-2xs font-semibold text-orange-500 mb-1">
											Parent changes (behind)
										</div>
										{#each diff.behindChanges as change}
											<div class="flex items-center gap-2 text-xs py-0.5">
												{#if change.kind === 'added'}
													<Plus class="w-3 h-3 text-green-500 shrink-0" />
												{:else if change.kind === 'removed'}
													<Minus class="w-3 h-3 text-red-500 shrink-0" />
												{:else}
													<Pencil class="w-3 h-3 text-yellow-500 shrink-0" />
												{/if}
												<span class="text-tertiary">{change.schemaName}.</span>
												<span class="font-medium">{change.tableName}</span>
												<span class="text-tertiary text-2xs grow">{operationSummary(change)}</span>
												<Button
													size="xs"
													variant="subtle"
													startIcon={{ icon: Eye }}
													onclick={() => openReview(change, diff, 'behind')}
												>
													Review
												</Button>
											</div>
										{/each}
									</div>
								{/if}
							</div>
						{/if}
					</ResizeTransitionWrapper>
				{/each}
			</div>
		{:else}
			<span class="text-xs text-secondary"> No changes detected </span>
		{/if}
	</div>
{/if}

<Drawer bind:open={drawerOpen} size="900px">
	{#if drawerChange && drawerDiff && drawerDirection}
		{@const yaml = getDiffYaml()}
		<DrawerContent
			on:close={() => (drawerOpen = false)}
			title="{drawerChange.schemaName}.{drawerChange.tableName} ({drawerDirection === 'ahead'
				? 'Fork → Parent'
				: 'Parent → Fork'})"
		>
			{#snippet actions()}
				<Button
					variant="accent"
					loading={migrationRunning}
					onclick={() => {
						if (drawerDirection === 'ahead') {
							confirmDeployOpen = true
						} else {
							runMigration()
						}
					}}
				>
					Run migration
				</Button>
			{/snippet}
			<div class="flex flex-col h-full">
				<Alert title="Changes to {drawerChange.tableName}" type="info">
					{#if drawerDirection == 'ahead'}
						You have made these changes in {currentWorkspaceId} that are not yet deployed in {parentWorkspaceId}.
					{:else if drawerDirection == 'behind'}
						These changes were made in {parentWorkspaceId} but the current workspace is not up to date.
					{/if}
				</Alert>
				<!-- Diff section -->
				<div style="height: 45%;">
					<div class="py-1.5 text-2xs font-semibold text-secondary">
						Schema diff (parent ↔ fork)
					</div>
					<div class="h-[calc(100%-28px)] border rounded-md overflow-clip">
						{#await import('$lib/components/DiffEditor.svelte')}
							<div class="flex items-center justify-center h-full">
								<Loader2 class="w-5 h-5 animate-spin" />
							</div>
						{:then Module}
							<Module.default
								open={true}
								automaticLayout
								className="h-full"
								defaultLang="yaml"
								defaultOriginal={yaml.original}
								defaultModified={yaml.modified}
								readOnly
							/>
						{/await}
					</div>
				</div>

				<!-- SQL migration section -->
				<div class="flex flex-col grow overflow-hidden mt-4">
					<div class="py-1.5 text-2xs font-semibold text-secondary"> SQL migration </div>
					<div class="grow overflow-clip rounded-md border">
						<SimpleEditor class="h-full" lang="sql" bind:code={migrationSql} />
					</div>
				</div>
			</div>
		</DrawerContent>
	{/if}
</Drawer>

<ConfirmationModal
	open={confirmDeployOpen}
	title="Deploy to parent workspace"
	confirmationText="Run migration"
	onConfirmed={async () => {
		confirmDeployOpen = false
		await runMigration()
	}}
	onCanceled={() => {
		confirmDeployOpen = false
	}}
>
	<p class="text-sm">
		This will run the following SQL on workspace <b>{parentWorkspaceId}</b>:
	</p>
	<pre
		class="mt-2 p-3 bg-surface-secondary rounded text-xs font-mono whitespace-pre-wrap max-h-60 overflow-auto"
		>{migrationSql}</pre
	>
</ConfirmationModal>

<Portal>
	<ConfirmationModal {...outOfOrderModal.props} />
</Portal>

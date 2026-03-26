<script module lang="ts">
	import type {
		TableEditorValues,
		TableEditorValuesColumn,
		TableEditorForeignKey
	} from '$lib/components/apps/components/display/dbtable/tableEditor'
	import {
		diffTableEditorValues,
		type AlterTableValues,
		makeAlterTableQueries
	} from '$lib/components/apps/components/display/dbtable/queries/alterTable'
	import type { GetDatatableFullSchemaResponse } from '$lib/gen'

	export type DatabaseSchema = Record<string, Record<string, TableEditorValues>>

	export function apiSchemaToEditorSchema(
		apiSchema: GetDatatableFullSchemaResponse
	): DatabaseSchema {
		const result: DatabaseSchema = {}
		for (const [schemaName, tables] of Object.entries(apiSchema)) {
			result[schemaName] = {}
			for (const [tableName, table] of Object.entries(tables as Record<string, any>)) {
				if (!table || typeof table !== 'object') continue
				result[schemaName][tableName] = {
					name: table.name ?? tableName,
					columns: (table.columns ?? []).map(
						(c: any): TableEditorValuesColumn => ({
							name: c.name,
							datatype: c.datatype,
							primaryKey: c.primary_key ?? c.primaryKey,
							defaultValue: c.default_value ?? c.defaultValue,
							nullable: c.nullable
						})
					),
					foreignKeys: (table.foreign_keys ?? table.foreignKeys ?? []).map(
						(fk: any): TableEditorForeignKey => ({
							targetTable: fk.target_table ?? fk.targetTable,
							columns: (fk.columns ?? []).map((col: any) => ({
								sourceColumn: col.source_column ?? col.sourceColumn,
								targetColumn: col.target_column ?? col.targetColumn
							})),
							onDelete: (fk.on_delete ?? fk.onDelete ?? 'NO ACTION') as
								| 'CASCADE'
								| 'SET NULL'
								| 'NO ACTION',
							onUpdate: (fk.on_update ?? fk.onUpdate ?? 'NO ACTION') as
								| 'CASCADE'
								| 'SET NULL'
								| 'NO ACTION',
							fk_constraint_name: fk.fk_constraint_name
						})
					),
					pk_constraint_name: table.pk_constraint_name
				}
			}
		}
		return result
	}

	export type TableDiff = {
		schemaName: string
		tableName: string
		kind: 'added' | 'removed' | 'modified'
		operations?: AlterTableValues
	}

	export type DatatableDiff = {
		datatableName: string
		aheadChanges: TableDiff[]
		behindChanges: TableDiff[]
		originalSchema: DatabaseSchema
		parentSchema: DatabaseSchema
		forkSchema: DatabaseSchema
	}

	export function diffDatabaseSchemas(
		original: DatabaseSchema,
		current: DatabaseSchema
	): TableDiff[] {
		const diffs: TableDiff[] = []
		const allSchemas = new Set([...Object.keys(original), ...Object.keys(current)])
		for (const schemaName of allSchemas) {
			const origTables = original[schemaName] ?? {}
			const currTables = current[schemaName] ?? {}
			const allTables = new Set([...Object.keys(origTables), ...Object.keys(currTables)])
			for (const tableName of allTables) {
				const origTable = origTables[tableName]
				const currTable = currTables[tableName]
				if (!origTable && currTable) {
					diffs.push({ schemaName, tableName, kind: 'added' })
				} else if (origTable && !currTable) {
					diffs.push({ schemaName, tableName, kind: 'removed' })
				} else if (origTable && currTable) {
					const currWithInitial: TableEditorValues = {
						...currTable,
						columns: currTable.columns.map((col) => ({ ...col, initialName: col.name }))
					}
					const diff = diffTableEditorValues(origTable, currWithInitial)
					if (diff.operations.length > 0) {
						diffs.push({ schemaName, tableName, kind: 'modified', operations: diff })
					}
				}
			}
		}
		return diffs
	}

	export function computeDatatableDiff(
		datatableName: string,
		originalSchema: DatabaseSchema,
		parentSchema: DatabaseSchema,
		forkSchema: DatabaseSchema
	): DatatableDiff {
		return {
			datatableName,
			behindChanges: diffDatabaseSchemas(originalSchema, parentSchema),
			aheadChanges: diffDatabaseSchemas(originalSchema, forkSchema),
			originalSchema,
			parentSchema,
			forkSchema
		}
	}

	export function generateMigrationSql(change: TableDiff, sourceSchema: DatabaseSchema): string {
		if (change.kind === 'modified' && change.operations) {
			const queries = makeAlterTableQueries(change.operations, 'postgresql', change.schemaName)
			if (queries.length === 0) return ''
			return (
				'-- Migration is auto-generated on a best-effort basis. You can adjust it here \n\n' +
				'BEGIN;\n' +
				queries.join('\n') +
				'\nCOMMIT;'
			)
		}
		if (change.kind === 'added') {
			const table = sourceSchema[change.schemaName]?.[change.tableName]
			if (!table) return ''
			const colDefs = table.columns
				.map((c) => {
					let def = `"${c.name}" ${c.datatype}`
					if (c.nullable === false) def += ' NOT NULL'
					if (c.defaultValue) def += ` DEFAULT ${c.defaultValue}`
					return def
				})
				.join(',\n  ')
			const pkCols = table.columns.filter((c) => c.primaryKey).map((c) => `"${c.name}"`)
			const pkLine = pkCols.length > 0 ? `,\n  PRIMARY KEY (${pkCols.join(', ')})` : ''
			return `BEGIN;\nCREATE TABLE "${change.schemaName}"."${change.tableName}" (\n  ${colDefs}${pkLine}\n);\nCOMMIT;`
		}
		if (change.kind === 'removed') {
			return `BEGIN;\nDROP TABLE IF EXISTS "${change.schemaName}"."${change.tableName}";\nCOMMIT;`
		}
		return ''
	}
</script>

<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { Loader2, ChevronDown, ChevronRight, Plus, Minus, Pencil, Eye } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { sendUserToast } from '$lib/toast'
	import { runScriptAndPollResult } from '$lib/components/jobs/utils'
	import YAML from 'yaml'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Alert from './common/alert/Alert.svelte'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
	}

	let { currentWorkspaceId, parentWorkspaceId }: Props = $props()

	let loading = $state(true)
	let error: string | undefined = $state(undefined)
	let diffs: DatatableDiff[] = $state([])
	let expandedDatatables: Set<string> = $state(new Set())

	// Drawer state
	let drawerOpen = $state(false)
	let drawerChange: TableDiff | undefined = $state(undefined)
	let drawerDiff: DatatableDiff | undefined = $state(undefined)
	let drawerDirection: 'ahead' | 'behind' | undefined = $state(undefined)
	let migrationSql = $state('')
	let migrationRunning = $state(false)
	let confirmDeployOpen = $state(false)

	async function loadDiffs() {
		loading = true
		error = undefined
		diffs = []
		try {
			const forkSettings = await WorkspaceService.getSettings({
				workspace: currentWorkspaceId
			})
			const datatables = forkSettings.datatable?.datatables ?? {}
			const forkedEntries = Object.entries(datatables).filter(([_, dt]) => dt.forked_from != null)
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
		migrationSql = generateMigrationSql(change, sourceSchema)
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
			await runScriptAndPollResult({
				workspace: targetWorkspace,
				requestBody: {
					args: { database: `datatable://${dtName}` },
					language: 'postgresql',
					content: migrationSql
				}
			})
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

			const forkSettings = await WorkspaceService.getSettings({
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

<h3 class="text-sm font-semibold">Datatable schema changes</h3>
{#if loading}
	<div class="flex items-center gap-2 text-xs text-tertiary py-2">
		<Loader2 class="w-4 h-4 animate-spin" /> Loading datatable diffs...
	</div>
{:else if error}
	<div class="text-xs text-red-500 py-2">Failed to load datatable diffs: {error}</div>
{:else if diffs.length > 0}
	<div class="flex flex-col gap-2">
		{#each diffs as diff}
			<div class="border rounded-md">
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
								<div class="text-2xs font-semibold text-blue-500 mb-1">Fork changes (ahead)</div>
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
			</div>
		{/each}
	</div>
{:else}
	<span class="text-xs text-secondary"> No changes detected </span>
{/if}

<Drawer bind:open={drawerOpen} size="900px">
	{#if drawerChange && drawerDiff && drawerDirection}
		{@const yaml = getDiffYaml()}
		<DrawerContent
			title="{drawerChange.schemaName}.{drawerChange.tableName} ({drawerDirection === 'ahead'
				? 'Fork → Parent'
				: 'Parent → Fork'})"
		>
			{#snippet actions()}
				<Button variant="default" onclick={() => (drawerOpen = false)}>Cancel</Button>
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
		This will run the following SQL on <b>{parentWorkspaceId}</b>:
	</p>
	<pre
		class="mt-2 p-3 bg-surface-secondary rounded text-xs font-mono whitespace-pre-wrap max-h-60 overflow-auto"
		>{migrationSql}</pre
	>
</ConfirmationModal>

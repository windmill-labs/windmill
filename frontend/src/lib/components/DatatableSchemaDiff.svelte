<script module lang="ts">
	import type {
		TableEditorValues,
		TableEditorValuesColumn,
		TableEditorForeignKey
	} from '$lib/components/apps/components/display/dbtable/tableEditor'
	import {
		diffTableEditorValues,
		type AlterTableValues
	} from '$lib/components/apps/components/display/dbtable/queries/alterTable'
	import type { GetDatatableFullSchemaResponse } from '$lib/gen'

	/** Full database schema: { schema_name: { table_name: TableEditorValues } } */
	export type DatabaseSchema = Record<string, Record<string, TableEditorValues>>

	/** Convert backend schema response to TableEditorValues format */
	export function apiSchemaToEditorSchema(
		apiSchema: GetDatatableFullSchemaResponse
	): DatabaseSchema {
		const result: DatabaseSchema = {}
		for (const [schemaName, tables] of Object.entries(apiSchema)) {
			result[schemaName] = {}
			for (const [tableName, table] of Object.entries(tables)) {
				result[schemaName][tableName] = {
					name: table.name,
					columns: table.columns.map(
						(c): TableEditorValuesColumn => ({
							name: c.name,
							datatype: c.datatype,
							primaryKey: c.primary_key,
							defaultValue: c.default_value,
							nullable: c.nullable
						})
					),
					foreignKeys: table.foreign_keys.map(
						(fk): TableEditorForeignKey => ({
							targetTable: fk.target_table,
							columns: fk.columns.map((col) => ({
								sourceColumn: col.source_column,
								targetColumn: col.target_column
							})),
							onDelete: (fk.on_delete as 'CASCADE' | 'SET NULL' | 'NO ACTION') ?? 'NO ACTION',
							onUpdate: (fk.on_update as 'CASCADE' | 'SET NULL' | 'NO ACTION') ?? 'NO ACTION',
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
	}

	/**
	 * Diff two full database schemas, returning per-table diffs.
	 * Uses diffTableEditorValues for tables that exist in both.
	 */
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
					// Set initialName on current columns so diffTableEditorValues can track renames
					const currWithInitial: TableEditorValues = {
						...currTable,
						columns: currTable.columns.map((col) => ({
							...col,
							initialName: col.name
						}))
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

	/**
	 * For a forked datatable, compute ahead/behind diffs by comparing:
	 * - (original, parent) → behind changes (parent drifted)
	 * - (original, fork) → ahead changes (fork drifted)
	 */
	export function computeDatatableDiff(
		datatableName: string,
		originalSchema: DatabaseSchema,
		parentSchema: DatabaseSchema,
		forkSchema: DatabaseSchema
	): DatatableDiff {
		return {
			datatableName,
			behindChanges: diffDatabaseSchemas(originalSchema, parentSchema),
			aheadChanges: diffDatabaseSchemas(originalSchema, forkSchema)
		}
	}
</script>

<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { Loader2, ChevronDown, ChevronRight, Plus, Minus, Pencil } from 'lucide-svelte'

	interface Props {
		currentWorkspaceId: string
		parentWorkspaceId: string
	}

	let { currentWorkspaceId, parentWorkspaceId }: Props = $props()

	let loading = $state(true)
	let error: string | undefined = $state(undefined)
	let diffs: DatatableDiff[] = $state([])
	let expandedDatatables: Set<string> = $state(new Set())

	async function loadDiffs() {
		loading = true
		error = undefined
		diffs = []

		try {
			// Get datatable config from the fork workspace to find forked datatables
			const forkSettings = await WorkspaceService.getSettings({
				workspace: currentWorkspaceId
			})
			const datatables = forkSettings.datatable?.datatables ?? {}

			const forkedEntries = Object.entries(datatables).filter(([_, dt]) => dt.forked_from != null)

			if (forkedEntries.length === 0) {
				loading = false
				return
			}

			// For each forked datatable, fetch schemas and compute diff
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
		if (expandedDatatables.has(name)) {
			expandedDatatables.delete(name)
		} else {
			expandedDatatables.add(name)
		}
		expandedDatatables = new Set(expandedDatatables)
	}

	function operationSummary(diff: TableDiff): string {
		if (diff.kind === 'added') return 'New table'
		if (diff.kind === 'removed') return 'Deleted table'
		const ops = diff.operations?.operations ?? []
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
</script>

{#if loading}
	<div class="flex items-center gap-2 text-xs text-tertiary py-2">
		<Loader2 class="w-4 h-4 animate-spin" /> Loading datatable diffs...
	</div>
{:else if error}
	<div class="text-xs text-red-500 py-2">Failed to load datatable diffs: {error}</div>
{:else if diffs.length > 0}
	<div class="flex flex-col gap-2">
		<h3 class="text-sm font-semibold">Datatable schema changes</h3>
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
								<div class="text-2xs font-semibold text-blue-500 mb-1"> Fork changes (ahead) </div>
								{#each diff.aheadChanges as change}
									<div class="flex items-center gap-2 text-xs py-0.5">
										{#if change.kind === 'added'}
											<Plus class="w-3 h-3 text-green-500" />
										{:else if change.kind === 'removed'}
											<Minus class="w-3 h-3 text-red-500" />
										{:else}
											<Pencil class="w-3 h-3 text-yellow-500" />
										{/if}
										<span class="text-tertiary">{change.schemaName}.</span>
										<span class="font-medium">{change.tableName}</span>
										<span class="text-tertiary text-2xs">{operationSummary(change)}</span>
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
											<Plus class="w-3 h-3 text-green-500" />
										{:else if change.kind === 'removed'}
											<Minus class="w-3 h-3 text-red-500" />
										{:else}
											<Pencil class="w-3 h-3 text-yellow-500" />
										{/if}
										<span class="text-tertiary">{change.schemaName}.</span>
										<span class="font-medium">{change.tableName}</span>
										<span class="text-tertiary text-2xs">{operationSummary(change)}</span>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}

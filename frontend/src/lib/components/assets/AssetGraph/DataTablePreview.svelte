<script lang="ts">
	// Single-table preview for a datatable asset. Shows the rows for the
	// specific table referenced by the asset path; if the table — or the
	// underlying datatable — doesn't exist yet, surfaces the relevant
	// empty state with a path forward (configure datatable / wait for the
	// upstream script to run). The full DBManager (with sidebar) is
	// available from the explore-asset button; in this pane the user
	// already picked the table they want to look at, so the sidebar would
	// just be noise.
	import DBTable from '$lib/components/DBTable.svelte'
	import { resource } from 'runed'
	import { workspaceStore } from '$lib/stores'
	import { loadAllTablesMetaData } from '$lib/components/apps/components/display/dbtable/metadata'
	import { dbTableOpsWithPreviewScripts } from '$lib/components/dbOps'
	import { WorkspaceService } from '$lib/gen'
	import type { DbInput } from '$lib/components/dbTypes'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import { AlertTriangle, Database, Loader2, Settings } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { base } from '$lib/base'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		// Asset path as parsed from `datatable://<datatable>/<table>` —
		// already stripped of the prefix. May be `<datatable>` only (no
		// table) or `<datatable>/<table>`.
		path: string | undefined
		// Bump this to force a re-fetch of the row data and metadata
		// (parallel to S3FilePreview's refreshKey). The asset detail pane
		// uses it after an upstream run completes so the table contents
		// get re-read without the user re-clicking.
		refreshKey?: any
		class?: string
	}
	let { path, refreshKey, class: className = '' }: Props = $props()

	// Default PG schema when the asset URI omits one. The asset URI shape
	// is `datatable://<db>/<table>` for the default schema and
	// `datatable://<db>/<schema>.<table>` for an explicit schema —
	// `parseDbInputFromAssetSyntax` returns `specificSchema: undefined` in
	// the first case, which we resolve to `public` here.
	const DEFAULT_SCHEMA = 'public'

	// Reuse the shared parser so explicit-schema URIs (like
	// `main/analytics.events`) are handled the same way the rest of the
	// codebase handles them, instead of treating the dot as part of the
	// table name.
	let parsed = $derived.by(() => {
		const p = path ?? ''
		const slash = p.indexOf('/')
		if (slash < 0)
			return { datatable: p, schema: undefined, table: undefined as string | undefined }
		const datatable = p.slice(0, slash)
		const tail = p.slice(slash + 1)
		// Delegate to the canonical parser so we pick up the same
		// `<schema>.<table>` convention as the rest of the asset stack.
		const dbInput = parseDbInputFromAssetSyntax(`datatable://${datatable}/${tail}`)
		const specificSchema =
			dbInput && 'specificSchema' in dbInput ? dbInput.specificSchema : undefined
		const specificTable = dbInput?.specificTable
		return {
			datatable,
			schema: specificSchema as string | undefined,
			table: specificTable as string | undefined
		}
	})

	// Workspace-configured datatables (the names in workspace_settings →
	// Data tables). If the parsed datatable isn't in this list, the
	// underlying connection / DB doesn't exist yet — there's nothing to
	// query, and the user needs to configure it before pipeline scripts
	// can write to it.
	let datatables = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return [] as string[]
			try {
				return (await WorkspaceService.listDataTables({ workspace: ws })).map((d) => d.name)
			} catch {
				return [] as string[]
			}
		}
	)

	let datatableExists = $derived((datatables.current ?? []).includes(parsed.datatable))

	// Resolve to the parsed schema when the URI was explicit, otherwise
	// fall back to the workspace default. This is what gets keyed into
	// loadAllTablesMetaData, so an explicit-schema URI like
	// `main/analytics.events` lands on the right entries.
	let effectiveSchema = $derived(parsed.schema ?? DEFAULT_SCHEMA)

	let input = $derived<DbInput | undefined>(
		parsed.datatable && datatableExists
			? {
					type: 'database',
					resourceType: 'postgresql',
					resourcePath: `datatable://${parsed.datatable}`,
					specificSchema: effectiveSchema,
					specificTable: parsed.table
				}
			: undefined
	)

	// Metadata for every table in the datatable. We query the lot rather
	// than just the one we care about because `loadAllTablesMetaData` is
	// what `dbTableOpsWithPreviewScripts` and the rest of the DB manager
	// expect — and the cost is one round-trip vs paying it later anyway.
	// `refreshKey` is included as a dep so a re-render after a producer
	// run picks up newly-created tables without the user reselecting.
	let colDefs = resource(
		() => [input, refreshKey],
		async ([_input]) => {
			if (!_input || !$workspaceStore) return undefined
			try {
				return await loadAllTablesMetaData($workspaceStore, _input)
			} catch {
				// Connection/permission errors look identical to "table
				// missing" from the user's POV inside the preview pane;
				// the full error is surfaced via the existing sendUserToast
				// path inside loadAllTablesMetaData.
				return undefined
			}
		}
	)

	// loadAllTablesMetaData keys columns by either `schema.table` (when
	// the DB has multiple schemas) or just `table`. We look up the
	// schema-qualified key first using the parsed/default schema, then
	// fall back to the bare key so the preview survives both shapes the
	// metadata loader returns.
	let tableColDefs = $derived.by(() => {
		if (!parsed.table || !colDefs.current) return undefined
		return colDefs.current[`${effectiveSchema}.${parsed.table}`] ?? colDefs.current[parsed.table]
	})

	let dbTableOps = $derived(
		input && tableColDefs && parsed.table && $workspaceStore
			? dbTableOpsWithPreviewScripts({
					input,
					tableKey: parsed.table,
					colDefs: tableColDefs,
					workspace: $workspaceStore
				})
			: undefined
	)
</script>

<div class={twMerge('flex flex-col h-full min-h-0 relative', className)}>
	{#if !parsed.datatable}
		<div class="p-3 text-xs text-tertiary">No datatable selected.</div>
	{:else if !parsed.table}
		<div class="p-3 text-xs text-tertiary">
			Pick a specific table — this asset only references the datatable
			<span class="font-mono">{parsed.datatable}</span>.
		</div>
	{:else if datatables.loading && !datatables.current}
		<div class="absolute inset-0 flex items-center justify-center text-tertiary">
			<Loader2 class="animate-spin" size={20} />
		</div>
	{:else if !datatableExists}
		<!-- The datatable name isn't in workspace settings yet — nothing
		     to query against. Send the user to the settings page rather
		     than creating one in-place because the choice of underlying
		     PG resource is non-trivial (custom instance DB vs external
		     resource) and the settings UI is the right place for it. -->
		<div class="absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 text-center">
			<Database size={28} class="text-tertiary" />
			<div class="flex flex-col gap-1">
				<span class="text-sm font-medium text-emphasis">Datatable not configured</span>
				<span class="text-2xs text-tertiary">
					The datatable
					<span class="font-mono">{parsed.datatable}</span>
					isn't set up in this workspace yet.
				</span>
			</div>
			<Button
				href="{base}/workspace_settings?tab=windmill_data_tables"
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: Settings }}
			>
				Configure datatable
			</Button>
		</div>
	{:else if colDefs.loading && !colDefs.current}
		<div class="absolute inset-0 flex items-center justify-center text-tertiary">
			<Loader2 class="animate-spin" size={20} />
		</div>
	{:else if !tableColDefs}
		<!-- Datatable exists but the specific table doesn't — most often
		     because no upstream pipeline script has run yet to create it. -->
		<div class="absolute inset-0 flex flex-col items-center justify-center gap-2 p-6 text-center">
			<AlertTriangle size={20} class="text-amber-500" />
			<span class="text-sm font-medium text-emphasis">Table doesn't exist yet</span>
			<span class="text-2xs text-tertiary max-w-xs">
				Nothing has been created at
				<span class="font-mono"
					>{parsed.datatable}/{parsed.schema ? `${parsed.schema}.` : ''}{parsed.table}</span
				>
				yet. It'll appear here once an upstream pipeline script writes to it.
			</span>
		</div>
	{:else if dbTableOps}
		<!-- Re-mount on refreshKey so DBTable's internal grid re-fetches
		     the row pages without us having to thread refresh hooks
		     through the table component. -->
		{#key refreshKey}
			<DBTable {dbTableOps} />
		{/key}
	{/if}
</div>

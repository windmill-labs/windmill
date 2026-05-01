<script lang="ts">
	// Inline preview for a datatable asset. Reuses the existing
	// DBManagerContent component (the same one the global Database Manager
	// drawer uses) so users get the same rich rows/schema view here that
	// they'd get clicking into a datatable from the Assets page.
	//
	// The asset path comes in as `<datatable>/<table>` (e.g. `main/event_summary`).
	// We split that into a `datatable://<datatable>` resourcePath + a
	// specificTable, mirroring what dbManagerDrawerModel.parse does for the
	// equivalent URL state.
	import DBManagerContent from '$lib/components/DBManagerContent.svelte'
	import type { DbInput } from '$lib/components/dbTypes'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		// Asset path as parsed from `datatable://<datatable>/<table>` —
		// already stripped of the prefix. May be `<datatable>` only (no
		// table) or `<datatable>/<table>`. We surface a small empty-state
		// hint when the table half is missing.
		path: string | undefined
		// Bump this to force a re-mount and re-fetch (parallel to
		// S3FilePreview's refreshKey). The asset detail pane uses it after
		// an upstream run completes so the table contents get re-read
		// without the user re-clicking.
		refreshKey?: any
		class?: string
	}

	let { path, refreshKey, class: className = '' }: Props = $props()

	// Default schema for datatable inputs is `public` — same default the
	// dbManagerDrawerModel applies when the URL omits it. The DB manager
	// resolves the workspace-specific Postgres schema at the connection
	// level, so we don't have to look it up here.
	const DEFAULT_SCHEMA = 'public'

	let parsed = $derived.by(() => {
		const p = path ?? ''
		const slash = p.indexOf('/')
		if (slash < 0) return { datatable: p, table: undefined as string | undefined }
		return { datatable: p.slice(0, slash), table: p.slice(slash + 1) }
	})

	let input = $derived<DbInput | undefined>(
		parsed.datatable
			? {
					type: 'database',
					resourceType: 'postgresql',
					resourcePath: `datatable://${parsed.datatable}`,
					specificSchema: DEFAULT_SCHEMA,
					specificTable: parsed.table
				}
			: undefined
	)

	// Selected schema/table for the manager. We pin to public + the parsed
	// table so the manager opens straight to the right rows; users can
	// still navigate via the sidebar if they want to see siblings.
	let selectedSchemaKey = $state<string | undefined>(DEFAULT_SCHEMA)
	let selectedTableKey = $state<string | undefined>(parsed.table)

	$effect(() => {
		// Re-pin selection when the path changes (e.g. user clicks a different
		// datatable asset in the graph).
		selectedSchemaKey = DEFAULT_SCHEMA
		selectedTableKey = parsed.table
	})
</script>

<div class={twMerge('flex flex-col h-full min-h-0', className)}>
	{#if !input}
		<div class="p-3 text-xs text-tertiary">No datatable selected.</div>
	{:else if !parsed.table}
		<div class="p-3 text-xs text-tertiary">
			Pick a specific table — this asset only references the datatable
			<span class="font-mono">{parsed.datatable}</span>.
		</div>
	{:else}
		<!-- Re-mount on refreshKey so DBManagerContent's internal resources
		     re-fetch (loadAllTablesMetaData, schemas, etc.) without us
		     having to thread refresh hooks through the manager. The {#key}
		     block is a coarse but effective way to do it. -->
		{#key refreshKey}
			<DBManagerContent {input} showRepl={false} bind:selectedSchemaKey bind:selectedTableKey />
		{/key}
	{/if}
</div>

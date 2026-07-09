<script lang="ts">
	// Captured output schema of a managed `// materialize` ducklake asset (parity
	// gap #2a). Read-only — schema is captured automatically after each
	// materialize.
	//
	// Two shapes, driven by `canEvolve`:
	// - Whole-table `replace` producer → the schema can change run-to-run, so this
	//   is a master-detail evolution history: version list (newest first, newest
	//   auto-selected) on the left, the selected version's columns on the right.
	// - `append` / `merge` / partitioned producer → the write INSERTs into a
	//   fixed-schema table, so the schema is pinned at first materialize; there's
	//   only ever one version, shown as a single current-schema table.
	import { resource } from 'runed'
	import { AssetService, type AssetSchemaVersion } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw, Lock } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
		// Whether the producer's strategy lets the schema evolve (whole-table
		// `replace`). False → single fixed-schema view. Defaults to true.
		canEvolve?: boolean
	}
	let { path, workspace, canEvolve = true }: Props = $props()

	// One column of a captured schema version (the generated `columns` element).
	type SchemaColumn = { name: string; type: string }

	let schemas = resource([() => workspace, () => path], async ([ws, p]) => {
		if (!ws || !p) return [] as AssetSchemaVersion[]
		return await AssetService.listAssetSchemas({ workspace: ws, path: p })
	})

	// The user's explicit pick (undefined until they click). Falls back to the
	// newest version when unset or when the pick is no longer in the list.
	let picked = $state<number | undefined>(undefined)
	let selected = $derived.by(() => {
		const list = schemas.current
		const inList = list?.find((s) => s.version === picked)
		return inList ?? list?.[0]
	})

	// Keep the version-list pane compact on wide panels (fixed-ish width rather
	// than a fixed fraction), mirroring the History tab.
	let paneWidth = $state(0)
	let listSize = $derived(
		paneWidth > 0 ? Math.max(24, Math.min(46, Math.round((220 / paneWidth) * 100))) : 34
	)
</script>

{#snippet columnsTable(cols: SchemaColumn[])}
	<table class="w-full text-xs">
		<thead class="text-tertiary text-left">
			<tr>
				<th class="font-medium pb-1 pr-2">Column</th>
				<th class="font-medium pb-1">Type</th>
			</tr>
		</thead>
		<tbody>
			{#each cols as col (col.name)}
				<tr class="border-t">
					<td class="py-1 pr-2 font-mono">{col.name}</td>
					<td class="py-1 font-mono text-tertiary">{col.type}</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/snippet}

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0">
		<span class="text-xs font-semibold text-secondary">Captured schema</span>
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: RefreshCw }}
			iconOnly
			onclick={() => schemas.refetch()}
			title="Refresh"
		/>
	</div>

	<div class="flex-1 min-h-0">
		{#if schemas.loading}
			<div class="flex items-center gap-2 text-tertiary text-xs p-3">
				<Loader2 size={14} class="animate-spin" /> Loading schema…
			</div>
		{:else if schemas.error}
			<p class="text-xs text-red-600 p-3">Failed to load: {schemas.error.message}</p>
		{:else if !schemas.current?.length}
			<p class="text-xs text-secondary p-3">
				No schema captured yet. The output schema is recorded automatically after a <span
					class="font-mono">// materialize</span
				> run.
			</p>
		{:else if !canEvolve}
			<!-- Fixed-schema producer (append / merge / partitioned): one schema,
			     no evolution. Show the current columns + why it can't change. -->
			<div class="h-full overflow-auto p-3">
				<div
					class="flex items-start gap-2 text-3xs text-tertiary mb-3 p-2 rounded bg-surface-secondary"
				>
					<Lock size={12} class="mt-0.5 shrink-0" />
					<span
						>This asset's schema is fixed — an append / merge / partitioned materialize INSERTs into
						a fixed-schema table, so the columns can't change run-to-run.</span
					>
				</div>
				{#if selected}
					{@render columnsTable(selected.columns)}
				{/if}
			</div>
		{:else}
			<div class="h-full" bind:clientWidth={paneWidth}>
				<Splitpanes class="!h-full">
					<Pane size={listSize} minSize={20}>
						<div class="h-full overflow-auto">
							{#each schemas.current as s, i (s.version)}
								<button
									type="button"
									class="w-full text-left px-3 py-2 border-b {selected?.version === s.version
										? 'border-blue-400 bg-blue-50 dark:border-blue-500 dark:bg-blue-950/30'
										: 'hover:bg-surface-hover'}"
									onclick={() => (picked = s.version)}
								>
									<div class="flex items-center justify-between gap-2">
										<span class="text-xs font-semibold">v{s.version}</span>
										{#if i === 0}
											<span
												class="px-1.5 py-0.5 rounded text-3xs font-medium bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300"
												>current</span
											>
										{/if}
									</div>
									<div class="text-3xs text-tertiary mt-0.5">
										{s.columns.length} column{s.columns.length === 1 ? '' : 's'}
										{#if s.snapshot_id != null}· snapshot {s.snapshot_id}{/if}
									</div>
									<div class="text-3xs text-tertiary">
										{new Date(s.captured_at).toLocaleString()}
									</div>
								</button>
							{/each}
						</div>
					</Pane>
					<Pane size={100 - listSize} minSize={30}>
						<div class="h-full overflow-auto p-3">
							{#if selected}
								{@render columnsTable(selected.columns)}
							{/if}
						</div>
					</Pane>
				</Splitpanes>
			</div>
		{/if}
	</div>
</div>

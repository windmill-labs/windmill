<script lang="ts">
	// One model, in the space the log panel otherwise occupies.
	//
	// The graph answers "what is in this project"; this answers "what is this
	// node" — the relation it writes, what dbt says about it, the transform
	// behind it and the rows it actually holds. It takes the whole bottom section
	// because a strip under the canvas could show none of that without scrolling,
	// and because selecting a node is a deliberate act with an obvious way back.
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { ClipboardCopy, Code2, FileCode2, Loader2, TableProperties, X } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import type { DbtAssetProvenance } from '$lib/components/assets/AssetGraph/types'
	import { previewDbtRows, type DbtPreview, type DbtPreviewBuffer } from './previewRows'
	import { nodeSelector } from './parseDbtRun'

	let {
		workspace,
		scriptPath,
		/** The relation this node writes, which is the asset path. */
		assetPath,
		dbt,
		/** Pins a preview to a deployed version, when the graph on screen is one.
		 *  Ignored when `buffer` is set — see it. */
		scriptHash,
		/** Set when the graph on screen was parsed from the editor, in which case
		 *  the rows have to come from that same buffer: the SQL shown above them
		 *  is the buffer's, and there may be no deployed version to run at all. */
		buffer,
		/** The run form's arguments: a `dbt show` is an invocation of this same
		 *  project, so a descriptor with a required `{{ }}` var needs them. Used
		 *  for a deployed graph; a buffer preview takes its parse's own instead. */
		args,
		/** Whether this model's file is in the bundle being edited. */
		fileInBundle = false,
		onOpenFile,
		onClose
	}: {
		workspace: string | undefined
		scriptPath: string
		assetPath: string
		dbt: DbtAssetProvenance
		scriptHash?: string | number
		buffer?: DbtPreviewBuffer
		args?: Record<string, unknown>
		fileInBundle?: boolean
		onOpenFile?: (path: string) => void
		onClose?: () => void
	} = $props()

	// Vars come from the snapshot when there is one: they decide `enabled`,
	// schemas and aliases, so they are what the graph on screen was drawn under,
	// and later ones could point `dbt show` at a relation it never described.
	let previewVars = $derived((((buffer ? buffer.args : args) as any)?.command as any)?.vars ?? {})
	// Placeholders come from the form, always. A parse runs `strict: false` and
	// drops a `{{ }}` only a run can fill, so a graph refreshed before the form
	// was filled carries empty ones — replaying those would fail every preview on
	// a value the run form is already holding.
	function placeholdersOf(a: Record<string, unknown> | undefined): Record<string, any> {
		const { command: _cmd, ...rest } = (a ?? {}) as Record<string, any>
		return rest
	}
	let previewPlaceholders = $derived(placeholdersOf(args))
	// Cached per model AND per inputs, so flipping between nodes does not re-spend
	// a worker slot, while an edited placeholder previews its own rows rather than
	// returning the last ones.
	let previews = $state<Record<string, DbtPreview>>({})
	let previewKey = $derived(
		`${dbt.unique_id}|${JSON.stringify(previewVars)}|${JSON.stringify(previewPlaceholders)}`
	)
	let preview = $derived(previews[previewKey])
	let previewing = $derived(preview != undefined && 'pending' in preview)
	// Which pane is showing. Keyed by the preview's own key so moving to another
	// node returns to the SQL without anything having to reset it.
	let rowsFor = $state<string | undefined>(undefined)
	let showRows = $derived(rowsFor === previewKey && preview != undefined)

	let destroyed = false
	$effect(() => () => (destroyed = true))

	async function runPreview() {
		if (!workspace) return
		const key = previewKey
		rowsFor = key
		const cached = previews[key]
		// A cached FAILURE must not stick: it would leave the button dead for that
		// model until reload. Only a result or an in-flight run blocks a re-run.
		if (cached != undefined && !('error' in cached)) return
		previews = { ...previews, [key]: { pending: true } }
		const next = await previewDbtRows({
			workspace,
			scriptPath,
			scriptHash,
			buffer,
			// Scoped to the node's package: a dependency package can ship a model
			// whose name the project also uses.
			model: nodeSelector(dbt.unique_id),
			vars: previewVars,
			args: previewPlaceholders,
			stillWanted: () => !destroyed
		})
		if (!next || destroyed) return
		previews = { ...previews, [key]: next }
	}

	// A column can hold an array or an object, whose default stringification is
	// `[object Object]` — which tells the reader nothing about the value.
	function cellText(v: unknown): string {
		if (v == undefined) return ''
		return typeof v === 'object' ? JSON.stringify(v) : String(v)
	}

	let columns = $derived(Object.entries(dbt.columns ?? {}))
	// `dbt show` SELECTs from the node's own relation and the worker intersects
	// the selector with `resource_type:model`, so offering it on a seed, snapshot
	// or source only ever produces a failed job.
	let previewable = $derived(dbt.resource_type === 'model')
	// Both channels reach relation identity — a var directly, a placeholder through
	// the descriptor var it fills — but only placeholders stay live, so only they
	// can make these rows describe another project. Kept apart because the two say
	// opposite things to a reader, and the pinned one is a guarantee, not a risk.
	let stalePlaceholders = $derived(
		buffer != undefined &&
			JSON.stringify(placeholdersOf(buffer.args)) !== JSON.stringify(previewPlaceholders)
	)
	let staleVars = $derived(
		buffer != undefined &&
			JSON.stringify(previewVars) !== JSON.stringify((args?.command as any)?.vars ?? {})
	)
</script>

<div class="h-full flex flex-col min-h-0">
	<div
		class="shrink-0 flex items-center gap-2 px-2 py-1 border-b bg-surface-secondary text-2xs text-secondary"
	>
		<span class="font-mono font-semibold text-primary truncate">{assetPath.split('/').pop()}</span>
		{#if dbt.materialized}
			<Badge color="gray" verySmall>{dbt.materialized}</Badge>
		{/if}
		{#if dbt.resource_type !== 'model'}
			<Badge color="blue" verySmall>{dbt.resource_type}</Badge>
		{/if}
		{#each dbt.tags ?? [] as t (t)}
			<Badge color="indigo" verySmall>{t}</Badge>
		{/each}
		<div class="ml-auto shrink-0 flex items-center gap-1">
			{#if previewable}
				{#if showRows && preview && !('error' in preview)}
					<Button
						unifiedSize="2xs"
						variant="subtle"
						startIcon={{ icon: Code2 }}
						on:click={() => (rowsFor = undefined)}
						title="Show the model's SQL">SQL</Button
					>
				{:else}
					<Button
						unifiedSize="2xs"
						variant="subtle"
						startIcon={{
							icon: previewing ? Loader2 : TableProperties,
							classes: previewing ? 'animate-spin' : undefined
						}}
						disabled={previewing}
						on:click={runPreview}
						title="Run `dbt show` against this model and display the rows"
						>{previewing ? 'Previewing…' : 'Preview rows'}</Button
					>
				{/if}
			{/if}
			{#if fileInBundle && dbt.original_file_path && onOpenFile}
				<Button
					unifiedSize="2xs"
					variant="subtle"
					startIcon={{ icon: FileCode2 }}
					on:click={() => onOpenFile?.(dbt.original_file_path!)}
					title="Open this model's file">Edit</Button
				>
			{/if}
			<Button
				unifiedSize="2xs"
				variant="subtle"
				startIcon={{ icon: ClipboardCopy }}
				iconOnly
				on:click={() => copyToClipboard(assetPath)}
				title="Copy the relation"
			/>
			<!-- Labelled and bordered rather than a subtle icon: this is how you get
			     the logs back, and an icon among four other icons did not read as the
			     way out. Clicking the canvas background does the same. -->
			<Button
				unifiedSize="2xs"
				variant="default"
				startIcon={{ icon: X }}
				on:click={() => onClose?.()}
				title="Close and deselect"
				btnClasses="ml-1">Close</Button
			>
		</div>
	</div>

	{#if stalePlaceholders}
		<div class="shrink-0 px-2 py-1 border-b text-2xs text-secondary bg-surface-secondary">
			The run arguments have changed since this graph was parsed, so these rows need not
			describe the models on screen — arguments reach schemas, aliases and which models exist
			at all. Refresh the models to draw and preview them under the current ones.
		</div>
	{:else if staleVars}
		<div class="shrink-0 px-2 py-1 border-b text-2xs text-secondary bg-surface-secondary">
			The run form's vars have changed since this graph was parsed. Rows are previewed under
			the vars it was parsed with, so they still describe the models on screen — refresh the
			models to draw and preview them under the current ones.
		</div>
	{/if}

	<div class="flex-1 min-h-0 overflow-auto">
		<div class="px-2 py-1.5 border-b text-2xs text-secondary flex flex-col gap-1">
			<div class="flex items-center gap-2">
				<span class="text-tertiary shrink-0">relation</span>
				<span class="font-mono truncate">{assetPath}</span>
			</div>
			{#if dbt.original_file_path}
				<div class="flex items-center gap-2">
					<span class="text-tertiary shrink-0">file</span>
					<span class="font-mono truncate">{dbt.original_file_path}</span>
				</div>
			{/if}
			{#if dbt.description}
				<div class="flex items-start gap-2">
					<span class="text-tertiary shrink-0">description</span>
					<span>{dbt.description}</span>
				</div>
			{/if}
		</div>

		{#if columns.length > 0 || (dbt.data_tests?.length ?? 0) > 0}
			<div class="px-2 py-1.5 border-b flex flex-col gap-1.5">
				{#if columns.length > 0}
					<div class="text-2xs">
						<div class="text-tertiary mb-0.5">columns declared</div>
						<div class="flex flex-col gap-0.5">
							{#each columns as [name, desc] (name)}
								<div class="flex gap-2">
									<span class="font-mono text-primary shrink-0">{name}</span>
									<span class="text-secondary truncate">{desc}</span>
								</div>
							{/each}
						</div>
						<!-- dbt's manifest carries no column-to-column edges, so this is a
						     declared column SET rather than lineage. -->
						<div class="text-tertiary mt-0.5">
							Declared metadata — dbt reports no column-level lineage.
						</div>
					</div>
				{/if}
				{#if (dbt.data_tests?.length ?? 0) > 0}
					<div class="text-2xs">
						<div class="text-tertiary mb-0.5">tests</div>
						<div class="flex flex-wrap gap-1">
							{#each dbt.data_tests ?? [] as t, i (i)}
								<Badge color={t.severity === 'warn' ? 'yellow' : 'red'} verySmall>
									{t.kind}{t.column ? ` · ${t.column}` : ''}
								</Badge>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}

		{#if showRows && preview}
			{#if 'error' in preview}
				<div class="p-2 text-2xs text-secondary">{preview.error}</div>
				{#if dbt.raw_code}
					<HighlightCode language="sql" code={dbt.raw_code} />
				{/if}
			{:else if 'pending' in preview}
				<div class="flex items-center gap-2 p-2 text-2xs text-secondary">
					<Loader2 size={12} class="animate-spin" />
					Running `dbt show` — this is a job, so it waits on a worker and the engine.
				</div>
			{:else}
				{@const cols = Object.keys(preview.rows[0] ?? {})}
				{#if cols.length === 0}
					<div class="p-2 text-2xs text-secondary">The model returned no rows.</div>
				{:else}
					<table class="w-full text-2xs font-mono">
						<thead class="sticky top-0 bg-surface-secondary text-secondary">
							<tr>
								{#each cols as c (c)}
									<th class="text-left font-semibold px-2 py-1 border-b">{c}</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each preview.rows as row, i (i)}
								<tr class="border-b border-surface-selected">
									{#each cols as c (c)}
										<td class="px-2 py-0.5 truncate max-w-56">{cellText(row[c])}</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
					<div class="px-2 py-1 text-3xs text-tertiary">
						{preview.rows.length} rows in {(preview.tookMs / 1000).toFixed(1)}s{preview.node
							? ` · ${preview.node}`
							: ''}
					</div>
				{/if}
			{/if}
		{:else if dbt.raw_code}
			<HighlightCode language="sql" code={dbt.raw_code} />
		{:else}
			<div class="p-2 text-2xs text-secondary">
				{dbt.resource_type === 'source'
					? 'A source is declared rather than built, so it has no transform of its own.'
					: 'No SQL stored for this node.'}
			</div>
		{/if}
	</div>
</div>

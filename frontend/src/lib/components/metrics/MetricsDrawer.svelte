<script lang="ts">
	import { Drawer, DrawerContent, Button, ClearableInput } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { DataMetricService, type DataMetric } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { SvelteSet } from 'svelte/reactivity'
	import {
		attachAliasFor,
		codeMentionsTable,
		composeMetricSql,
		DEFAULT_LAKE_ALIAS,
		pickAttachAlias,
		splitTablePath
	} from './metricSql'
	import { copyToClipboard } from '$lib/utils'
	import { Copy, Play } from 'lucide-svelte'
	import SqlRepl from '$lib/components/SqlRepl.svelte'
	import SimpleAgTable from '$lib/components/SimpleAgTable.svelte'

	let {
		getCode,
		onInsert,
		workspace
	}: {
		getCode: () => string
		onInsert: (sql: string) => void
		workspace?: string
	} = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let drawer: Drawer | undefined = $state()
	let entries = $state<DataMetric[]>([])
	let loading = $state(false)
	let loaded = $state(false)
	let selectedTable = $state<string | undefined>(undefined)
	// Snapshotted on open so the composed snippet is stable while the drawer is up.
	let codeAtOpen = $state('')
	// Selections are held per declaration, not per name: two live producers may
	// declare the same name on one table, and keying by name alone would make
	// toggling either one select both. Selections survive a table switch, which is
	// harmless because `sql` only ever reads the current table's declarations.
	const selectedMeasures = new SvelteSet<string>()
	const selectedDims = new SvelteSet<string>()

	function idOf(m: DataMetric): string {
		return `${m.script_path}|${m.name}`
	}

	// Name filters, shown only past a threshold so a small catalog stays a plain,
	// scannable list. The table filter is cleared implicitly by narrowing; the metric
	// filter is reset on table switch (a name from one table rarely fits another).
	let tableFilter = $state('')
	let metricFilter = $state('')
	const TABLE_FILTER_MIN = 12
	const METRIC_FILTER_MIN = 8

	function nameMatches(name: string, filter: string): boolean {
		return name.toLowerCase().includes(filter.trim().toLowerCase())
	}

	let tables = $derived([...new Set(entries.map((e) => e.table_path))])
	let shownTables = $derived(
		tableFilter.trim() ? tables.filter((t) => nameMatches(t, tableFilter)) : tables
	)
	let current = $derived(entries.filter((e) => e.table_path === selectedTable))
	// Full table sets drive selection and SQL composition; the `shown*` derivations
	// only narrow what renders, so filtering never drops an already-checked metric.
	let measures = $derived(current.filter((m) => m.kind === 'measure'))
	let dimensions = $derived(current.filter((m) => m.kind === 'dimension'))
	let shownMeasures = $derived(
		metricFilter.trim() ? measures.filter((m) => nameMatches(m.name, metricFilter)) : measures
	)
	let shownDimensions = $derived(
		metricFilter.trim() ? dimensions.filter((d) => nameMatches(d.name, metricFilter)) : dimensions
	)

	function build(existingAlias: string | undefined, attachAs: string): string {
		if (!selectedTable) return ''
		const ms = measures.filter((m) => selectedMeasures.has(idOf(m)))
		const ds = dimensions.filter((d) => selectedDims.has(idOf(d)))
		if (ms.length === 0 && ds.length === 0) return ''
		return composeMetricSql({
			tablePath: selectedTable,
			measures: ms,
			dimensions: ds,
			existingAlias,
			attachAs
		})
	}

	// Only consulted when inserting an ATTACH of our own, which happens only if the
	// script has no alias for this lake. Every candidate is checked, since a blind
	// fallback can land on a name the script has already attached something under.
	let insertAttachAs = $derived.by(() => {
		const parts = selectedTable ? splitTablePath(selectedTable) : undefined
		if (!parts) return DEFAULT_LAKE_ALIAS
		return pickAttachAlias(codeAtOpen, parts.lake)
	})

	// The alias this script already attaches the lake under, if any.
	let existingAlias = $derived.by(() => {
		const parts = selectedTable ? splitTablePath(selectedTable) : undefined
		return parts ? attachAliasFor(codeAtOpen, parts.lake) : undefined
	})

	// Shown and copied: a complete standalone query, attaching the lake under `dl`.
	// Inserting adapts it instead, because repeating an ATTACH the script already
	// has would not run.
	let sql = $derived(build(undefined, DEFAULT_LAKE_ALIAS))
	let insertSql = $derived(build(existingAlias, insertAttachAs))
	let insertAdapts = $derived(!!existingAlias && insertSql !== sql)

	// The REPL wraps whatever it runs in `ATTACH 'ducklake://<lake>' AS dl; USE dl;`,
	// so it wants the query qualified with `dl` and carrying no ATTACH of its own.
	let replSql = $derived(build(DEFAULT_LAKE_ALIAS, DEFAULT_LAKE_ALIAS))
	let lake = $derived(selectedTable ? splitTablePath(selectedTable)?.lake : undefined)
	let repl: SqlRepl | undefined = $state()
	// Results tagged with the query that produced them (the REPL reports which code
	// it ran). Staleness is then a render check (`preview.sql === replSql`): when the
	// selection changes the shown results no longer match and vanish, and a late
	// response for a superseded query is discarded rather than shown under the new
	// selection — no effect needed to clear it.
	let preview = $state<{ sql: string; data: Record<string, any>[] } | undefined>(undefined)

	function runPreview() {
		preview = undefined
		repl?.setCode(replSql)
	}

	// One page's worth; the endpoint caps `per_page` at 1000. Larger catalogs are
	// walked page by page via the keyset cursor below.
	const PER_PAGE = 1000
	// Hard stop against a runaway loop; each page advances the cursor, so a real
	// catalog exhausts (the server drops `next_cursor`) long before this.
	const MAX_PAGES = 1000

	// The trigger lives in the editor bar, which places it either among the buttons
	// or inside the compact Helpers menu.
	export async function open() {
		codeAtOpen = getCode()
		drawer?.openDrawer()
		loading = true
		try {
			const all: DataMetric[] = []
			// Resume each page from the cursor the server hands back; every row it
			// returns is authorized, so stop once no cursor comes back.
			let cursor:
				| { table_path: string; kind: string; name: string; script_path: string }
				| undefined
			for (let i = 0; i < MAX_PAGES; i++) {
				const res = await DataMetricService.listDataMetrics({
					workspace: ws!,
					perPage: PER_PAGE,
					cursorTable: cursor?.table_path,
					cursorKind: cursor?.kind,
					cursorName: cursor?.name,
					cursorScript: cursor?.script_path
				})
				all.push(...res.metrics)
				if (!res.next_cursor) break
				cursor = res.next_cursor
			}
			entries = all
		} catch (err) {
			console.error("Couldn't load declared metrics", err)
			entries = []
		} finally {
			loading = false
			loaded = true
		}
		if (!selectedTable || !tables.includes(selectedTable)) {
			// Prefer a table the open script already references.
			selectedTable = tables.find((t) => codeMentionsTable(codeAtOpen, t)) ?? tables[0]
		}
	}

	function toggle(set: SvelteSet<string>, id: string) {
		if (set.has(id)) {
			set.delete(id)
		} else {
			set.add(id)
		}
	}

	function definition(m: DataMetric): string {
		return m.filter ? `${m.expr} where ${m.filter}` : m.expr
	}

	// Counts per table in one pass, so rendering T tables over N declarations is
	// O(N) rather than O(T*N) filters.
	let countByTable = $derived.by(() => {
		const counts = new Map<string, { m: number; d: number }>()
		for (const e of entries) {
			const c = counts.get(e.table_path) ?? { m: 0, d: 0 }
			if (e.kind === 'measure') c.m++
			else c.d++
			counts.set(e.table_path, c)
		}
		return counts
	})
</script>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent title="Metrics" on:close={() => drawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-tertiary">
				Declared with <code>// measure</code> and <code>// dimension</code> on the script that materializes
				the table. Inserting writes plain SQL you can edit.
			</p>

			{#if loading}
				<span class="text-xs text-tertiary">Loading…</span>
			{:else if loaded && tables.length === 0}
				<div class="flex flex-col gap-1 rounded-md border p-3">
					<span class="text-sm">No metrics declared yet</span>
					<span class="text-xs text-tertiary">
						Add <code>// measure revenue = sum(amount) where not is_refund</code> next to a script's
						<code>// materialize</code> annotation, then deploy it.
					</span>
				</div>
			{/if}

			{#if tables.length > 0}
				<div>
					<div class="mb-1 text-xs font-medium text-secondary">
						{tables.length === 1 ? 'Table' : `Tables with metrics (${tables.length})`}
					</div>
					{#if tables.length > TABLE_FILTER_MIN}
						<div class="mb-1">
							<ClearableInput
								bind:value={tableFilter}
								placeholder="Filter tables…"
								inputClass="text-xs"
							/>
						</div>
					{/if}
					<!-- Laid out as visible choices rather than a dropdown: browsing what
					     else is declared is the point, and a filter appears once there are
					     enough tables to warrant one. -->
					<div class="flex flex-wrap gap-1">
						{#each shownTables as t (t)}
							<Button
								size="xs"
								variant={t === selectedTable ? 'accent' : 'subtle'}
								onclick={() => {
									selectedTable = t
									metricFilter = ''
								}}
							>
								<span class="font-mono text-xs">{t}</span>
								<span class="ml-1 text-xs opacity-60">
									{countByTable.get(t)?.m ?? 0}m / {countByTable.get(t)?.d ?? 0}d
								</span>
							</Button>
						{/each}
					</div>
					{#if shownTables.length === 0}
						<span class="text-xs text-tertiary">No table matches “{tableFilter}”</span>
					{/if}
				</div>
			{/if}

			{#if measures.length + dimensions.length > METRIC_FILTER_MIN}
				<ClearableInput
					bind:value={metricFilter}
					placeholder="Filter measures and dimensions by name…"
					inputClass="text-xs"
				/>
			{/if}

			{#if shownMeasures.length > 0}
				<div>
					<div class="mb-1 text-xs font-medium text-secondary">Measures</div>
					<div class="flex flex-col gap-1">
						{#each shownMeasures as m (idOf(m))}
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									options={{ right: m.name }}
									checked={selectedMeasures.has(idOf(m))}
									on:change={() => toggle(selectedMeasures, idOf(m))}
								/>
								<span class="truncate font-mono text-xs text-tertiary" title={definition(m)}>
									= {definition(m)}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			{#if shownDimensions.length > 0}
				<div>
					<div class="mb-1 text-xs font-medium text-secondary">Group by</div>
					<div class="flex flex-col gap-1">
						{#each shownDimensions as d (idOf(d))}
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									options={{ right: d.name }}
									checked={selectedDims.has(idOf(d))}
									on:change={() => toggle(selectedDims, idOf(d))}
								/>
								<span class="truncate font-mono text-xs text-tertiary" title={d.expr}>
									= {d.expr}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			{#if metricFilter.trim() && shownMeasures.length === 0 && shownDimensions.length === 0 && measures.length + dimensions.length > 0}
				<span class="text-xs text-tertiary">No metric matches “{metricFilter}”</span>
			{/if}

			{#if sql}
				<div>
					<div class="mb-1 flex items-center justify-between">
						<span class="text-xs font-medium text-secondary">SQL</span>
						<div class="flex items-center gap-1">
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: Play }}
								disabled={!lake || !replSql}
								onclick={runPreview}>Try it</Button
							>
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: Copy }}
								onclick={() => copyToClipboard(sql)}>Copy</Button
							>
						</div>
					</div>
					<pre class="overflow-x-auto rounded-md bg-surface-secondary p-2 text-xs whitespace-pre"
						>{sql}</pre
					>
					{#if insertAdapts}
						<div class="mt-1 text-xs text-tertiary">
							Inserting reuses this script's existing <span class="font-mono">{existingAlias}</span>
							attachment and leaves out the ATTACH.
						</div>
					{/if}
				</div>
			{/if}

			{#if lake}
				<div>
					<div class="mb-1 text-xs font-medium text-secondary">Try it</div>
					<div class="h-64 rounded-md border">
						<SqlRepl
							bind:this={repl}
							input={{ type: 'ducklake', ducklake: lake }}
							workspace={ws}
							onData={(d, ranCode) => (preview = { sql: ranCode, data: d })}
						/>
					</div>
					{#if preview && preview.sql === replSql}
						<!-- ag-grid measures itself against its container, so the height
						     has to be explicit or the table renders collapsed. -->
						<div class="mt-2 h-64">
							<SimpleAgTable data={preview.data} />
						</div>
					{/if}
				</div>
			{/if}
		</div>

		{#snippet actions()}
			<Button disabled={!insertSql} onclick={() => (onInsert(insertSql), drawer?.closeDrawer())}>
				Insert
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

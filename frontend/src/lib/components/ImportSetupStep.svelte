<script lang="ts">
	import { ResourceService, WorkspaceService } from '$lib/gen'
	import { ArrowLeft, Check, CheckCircle2, Database, KeyRound, Loader2, X } from 'lucide-svelte'
	import { fly } from 'svelte/transition'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Button } from '$lib/components/common'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import {
		convertDataTableSettingsFromBackend,
		convertDataTableSettingsToBackend,
		type DataTableSettingsType
	} from '$lib/components/workspaceSettings/DataTableSettings.svelte'
	import { applyOneMigration } from '$lib/components/workspaceSettings/projectInstall'
	import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
	import { isCustomInstanceDbEnabled } from '$lib/components/workspaceSettings/utils.svelte'
	import { randomUUID } from '$lib/utils/uuid'
	import { sendUserToast } from '$lib/toast'

	// The last step, and the only optional one: it exists when the project's data
	// tables are not configured in the destination. The import has already run —
	// everything here is the part it could not do, because a data table is a named
	// database connection the workspace owns, not something an import can invent.
	//
	// Self-sufficient from `workspace` + `slug`: it re-fetches the export rather than
	// reading the executor, so reloading the page on this step still works. The plan
	// in the URL stays the whole state.

	interface Props {
		workspace: string
		slug: string
		onSkip: () => void
		onFinish: () => void
		onBack?: () => void
	}

	let { workspace, slug, onSkip, onFinish, onBack }: Props = $props()

	type Row = {
		name: string
		migrations: ProjectMigration[]
		/** Postgres resource backing it, or an instance database when that is enabled. */
		resourcePath: string | undefined
		status: 'unconfigured' | 'saving' | 'running' | 'done' | 'failed'
		error?: string
	}

	/** A resource the project shipped that needed filling in. */
	type Blank = {
		path: string
		resourceType: string
		/** Required fields the type declares and the value does not have yet. */
		missing: string[]
		/** Filled in since this step opened. The row stays — it is a checklist, and a
		 * line that vanishes when you complete it reads as something going wrong. */
		done: boolean
		/** Plays the confirmation flash once, right after the save that flipped it. */
		justSaved: boolean
	}

	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let rows = $state<Row[]>([])
	let blanks = $state<Blank[]>([])
	let projectResources: { path: string; resource_type: string }[] = []
	let working = $state(false)
	let resourceEditor: ResourceEditorDrawer | undefined = $state(undefined)

	const pendingTables = $derived(rows.filter((r) => r.status !== 'done'))
	const canRun = $derived(
		!working && pendingTables.length > 0 && pendingTables.every((r) => !!r.resourcePath)
	)

	/** Which data tables the project needs that the destination does not have yet. */
	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await fetch(
				`/api/w/${encodeURIComponent(workspace)}/hub/projects/${encodeURIComponent(slug)}/export`
			)
			if (!res.ok) throw new Error(`the hub proxy answered ${res.status}`)
			const exportData = (await res.json()) as {
				migrations?: ProjectMigration[]
				resources?: { path: string; resource_type: string }[]
			}
			const enabled = (exportData.migrations ?? []).filter(
				(m) => m.enabled && (m.sql ?? '').trim() !== ''
			)
			const present = new Set(
				(await WorkspaceService.listDataTables({ workspace })).map((d) => d.name)
			)
			const missing = [...new Set(enabled.map((m) => m.datatable_name))].filter(
				(n) => !present.has(n)
			)
			rows = missing.map((name) => ({
				name,
				migrations: enabled.filter((m) => m.datatable_name === name),
				resourcePath: undefined,
				status: 'unconfigured' as const
			}))
			projectResources = exportData.resources ?? []
			await refreshBlanks()
		} catch (e: any) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	/**
	 * Resources the import created but could not fill. Every shipped resource arrives
	 * as a stub — the hub never publishes resource values, they are credentials — so
	 * this is not "which ones are empty" but "which ones are *still* empty": a
	 * re-import leaves an already-filled resource alone.
	 *
	 * The type's schema names the required fields, so the row can say what is missing
	 * rather than just that something is. A type we cannot read still counts as blank
	 * when the value is empty; it just lists no field names.
	 */
	async function findBlankResources(
		resources: { path: string; resource_type: string }[]
	): Promise<Blank[]> {
		const out: Blank[] = []
		for (const r of resources) {
			let value: any
			try {
				value = (await ResourceService.getResource({ workspace, path: r.path }))?.value
			} catch {
				continue // Not there — the import reported that failure already.
			}
			const filled = new Set(
				value && typeof value === 'object'
					? Object.entries(value)
							.filter(([, v]) => v !== undefined && v !== null && v !== '')
							.map(([k]) => k)
					: []
			)
			let required: string[] = []
			try {
				const schema = (await ResourceService.getResourceType({ workspace, path: r.resource_type }))
					?.schema as { required?: string[] } | undefined
				required = schema?.required ?? []
			} catch {}
			const missing = required.filter((k) => !filled.has(k))
			if (missing.length > 0 || filled.size === 0) {
				out.push({
					path: r.path,
					resourceType: r.resource_type,
					missing,
					done: false,
					justSaved: false
				})
			}
		}
		return out
	}

	/**
	 * Re-read the resources and settle each row's state. Rows are never dropped once
	 * listed: the first pass decides what the checklist contains, and every pass after
	 * it only moves a row from outstanding to done.
	 */
	async function refreshBlanks(): Promise<void> {
		const fresh = await findBlankResources(projectResources)
		const stillBlank = new Map(fresh.map((b) => [b.path, b]))
		if (blanks.length === 0) {
			blanks = fresh
			return
		}
		blanks = blanks.map((b) => {
			const f = stillBlank.get(b.path)
			if (f) return { ...b, missing: f.missing, done: false, justSaved: false }
			return { ...b, missing: [], done: true, justSaved: !b.done }
		})
		// The flash is a one-shot; clear it so a later refresh does not replay it.
		for (const b of blanks) {
			if (!b.justSaved) continue
			setTimeout(() => {
				const row = blanks.find((x) => x.path === b.path)
				if (row) row.justSaved = false
			}, 1500)
		}
	}

	$effect(() => {
		void load()
	})

	/**
	 * Configure every named data table in one write, then run each one's migrations.
	 * The config is read back and merged rather than replaced: `editDataTableConfig`
	 * takes the whole settings object, so sending only ours would delete any the
	 * workspace already has.
	 */
	async function configureAndRun() {
		working = true
		try {
			const current = await WorkspaceService.getSettings({ workspace })
			const settings: DataTableSettingsType = convertDataTableSettingsFromBackend(current.datatable)
			for (const row of rows) {
				if (row.status === 'done') continue
				settings.dataTables.push({
					id: randomUUID(),
					name: row.name,
					database: {
						resource_type: $isCustomInstanceDbEnabled ? 'instance' : 'postgresql',
						resource_path: row.resourcePath
					}
				})
				row.status = 'saving'
			}
			await WorkspaceService.editDataTableConfig({
				workspace,
				requestBody: {
					settings: convertDataTableSettingsToBackend(settings),
					renames: [],
					deleted_datatables: []
				}
			})

			for (const row of rows) {
				if (row.status === 'done') continue
				row.status = 'running'
				try {
					for (const m of row.migrations) await applyOneMigration(workspace, slug, m)
					row.status = 'done'
					row.error = undefined
				} catch (e: any) {
					row.status = 'failed'
					row.error = e?.body ?? e?.message ?? String(e)
				}
			}
		} catch (e: any) {
			const detail = e?.body ?? e?.message ?? String(e)
			for (const row of rows) if (row.status === 'saving') row.status = 'failed'
			sendUserToast(`Could not save the data table settings: ${detail}`, true)
		} finally {
			working = false
		}
	}
</script>

<div class="flex flex-col gap-4">
	<div>
		<h2 class="text-sm font-semibold text-emphasis">Finish setting up</h2>
		<p class="mt-0.5 text-xs text-secondary">
			The project is imported. What is left is the part it could not bring with it — connections and
			credentials this workspace has to supply.
		</p>
	</div>

	{#if loading}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Loader2 size={14} class="animate-spin" /> Checking what this project needs…
		</div>
	{:else if loadError}
		<Alert type="warning" title="Could not check the project's data tables" size="xs">
			{loadError}. You can finish and configure them later in Workspace settings → Data tables.
		</Alert>
	{:else}
		<ul class="flex flex-col gap-3">
			{#each rows as row (row.name)}
				<li class="rounded-md border border-border-light p-3">
					<div class="flex items-center gap-2">
						<Database size={14} class="shrink-0 text-secondary" />
						<span class="font-mono text-xs text-emphasis">{row.name}</span>
						{#if row.status === 'done'}
							<Check size={13} class="text-emerald-600" />
							<span class="text-xs text-secondary">
								configured · {row.migrations.length} migration{row.migrations.length === 1
									? ''
									: 's'} run
							</span>
						{:else if row.status === 'saving' || row.status === 'running'}
							<Loader2 size={13} class="animate-spin text-blue-500" />
							<span class="text-xs text-secondary">
								{row.status === 'saving' ? 'saving settings…' : 'running migrations…'}
							</span>
						{:else if row.status === 'failed'}
							<X size={13} class="text-red-500" />
							<span class="text-xs text-red-500">{row.error}</span>
						{:else}
							<span class="text-xs text-tertiary">not configured</span>
						{/if}
					</div>

					{#if row.status !== 'done'}
						<div class="mt-2">
							<span class="mb-1 block text-xs font-normal text-secondary">Database</span>
							<ResourcePicker
								bind:value={row.resourcePath}
								resourceType="postgresql"
								disabled={working}
								placeholder="Pick a Postgres resource"
							/>
						</div>
					{/if}
				</li>
			{/each}
		</ul>

		{#if blanks.length > 0}
			<div class="flex flex-col gap-2">
				<span class="text-xs font-normal text-secondary">Credentials to fill ({blanks.length})</span
				>
				<ul class="flex flex-col gap-1.5">
					{#each blanks as b (b.path)}
						<li
							class="flex items-center gap-2 rounded-md border border-border-light px-3 py-2 text-xs"
						>
							{#if b.done}
								<Check size={14} class="shrink-0 text-emerald-600" />
							{:else}
								<KeyRound size={14} class="shrink-0 text-secondary" />
							{/if}
							<span class="min-w-0 flex-1">
								<span class="block truncate font-mono text-emphasis">{b.path}</span>
								<span class="block truncate text-tertiary">
									{b.done
										? b.resourceType
										: `${b.resourceType}${
												b.missing.length > 0 ? ` · missing ${b.missing.join(', ')}` : ''
											}`}
								</span>
							</span>
							<!-- The confirmation flash is the one from SaveButton: the save itself
							     happens in the resource drawer, so only the overlay is reused here.
							     The button stays live either way — "Saved" is a state, not a dead end. -->
							<div class="relative overflow-hidden rounded-md">
								<Button
									variant={b.done ? 'subtle' : 'accent'}
									unifiedSize="sm"
									disabled={working}
									onClick={() => resourceEditor?.initEdit(b.path)}
								>
									{b.done ? 'Saved' : 'Fill in'}
								</Button>
								{#if b.justSaved}
									<div
										class="absolute inset-0 flex items-center justify-center rounded-md bg-green-200 dark:bg-green-800"
										transition:fly={{ y: -10, duration: 300 }}
									>
										<CheckCircle2 class="h-5 w-5 text-green-700 dark:text-green-300" />
									</div>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		<Alert type="warning" title="You can skip this" size="xs" collapsible>
			The project's apps and flows will fail wherever they read something that is still
			unconfigured. Everything else it imported works either way, and you can come back to this from
			the workspace at any time.
		</Alert>
	{/if}

	<div class="mt-2 flex items-center justify-between">
		{#if onBack}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: ArrowLeft }}
				disabled={working}
				onClick={onBack}
			>
				Back
			</Button>
		{:else}
			<span></span>
		{/if}
		<div class="flex items-center gap-2">
			<!-- Only a data table needs an action here: it is the one thing this step can
			     do for the user. Credentials are filled in their own editor, so once the
			     tables are handled the way out is simply to finish — leaving some blank is
			     the skip, which the warning above spells out. -->
			{#if pendingTables.length > 0 && !loading && !loadError}
				<Button variant="subtle" unifiedSize="sm" disabled={working} onClick={onSkip}>
					Skip for now
				</Button>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!canRun}
					loading={working}
					onClick={configureAndRun}
				>
					Configure and run migrations
				</Button>
			{:else}
				<Button variant="accent" unifiedSize="sm" disabled={working} onClick={onFinish}>
					Finish setup →
				</Button>
			{/if}
		</div>
	</div>
</div>

<!-- The destination is not the workspace the app is in until the run switches to it,
     so the editor is told which one explicitly. -->
<ResourceEditorDrawer
	bind:this={resourceEditor}
	{workspace}
	onSaved={() => void load()}
	onRestored={() => void load()}
/>

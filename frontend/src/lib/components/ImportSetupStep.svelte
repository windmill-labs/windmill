<script lang="ts">
	import { ResourceService, WorkspaceService } from '$lib/gen'
	import { ArrowLeft, Check, Database, Loader2, X } from 'lucide-svelte'
	import { tick } from 'svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Button } from '$lib/components/common'
	import AddDataTableWizard from '$lib/components/workspaceSettings/AddDataTableWizard.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import { SettingService } from '$lib/gen'
	import { resource } from 'runed'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import ImportSetupRow from '$lib/components/ImportSetupRow.svelte'
	import AppConnectDrawer from '$lib/components/AppConnectDrawer.svelte'
	import { OauthService } from '$lib/gen'
	import { resourceTypeDisplayName } from '$lib/components/resourceTypeDisplay'
	import { applyOneMigration } from '$lib/components/workspaceSettings/projectInstall'
	import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
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
		status: 'unconfigured' | 'running' | 'done' | 'failed'
		error?: string
		/** Plays the confirmation flash once, right after the run that configured it. */
		justSaved: boolean
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
	/** Rows the user has not dealt with, of either kind. */
	const outstanding = $derived(pendingTables.length + blanks.filter((b) => !b.done).length)

	// The wizard needs the instance-database pool and a confirmation host; the settings
	// page owns them there, so this step owns them here.
	// Which resource types this instance has an OAuth client for. A resource whose type is
	// in here can be connected instead of hand-filled, which for an OAuth type is the
	// difference between clicking Connect and pasting a token that expires in an hour.
	// Empty when no superadmin has configured any client — then every row falls back to
	// the editor, which is the only thing that would work anyway.
	const oauthConnects = resource(
		() => workspace,
		async () => {
			try {
				return (await OauthService.listOauthConnects()).map((c) => c.name)
			} catch {
				return []
			}
		}
	)
	const connectable = $derived(new Set(oauthConnects.current ?? []))
	let appConnect: AppConnectDrawer | undefined = $state(undefined)

	const customInstanceDbs = resource([() => workspace], SettingService.listCustomInstanceDbs)
	const confirmationModal = createAsyncConfirmationModal()
	let wizardOpen = $state(false)
	let wizard = $state<AddDataTableWizard | undefined>(undefined)
	let wizardFor = $state<string | undefined>(undefined)
	let configuredNames = $state<{ name: string; resourcePath: string | undefined }[]>([])

	function openWizard(name: string) {
		wizardFor = name
		// `open()`, not `opened = true`: only the method runs the wizard's own reset, which
		// is what applies `initialName` and clears whatever a previous run left behind.
		wizardOpen = true
		void tick().then(() => wizard?.open())
	}

	function defaultInstanceDbName(): string {
		const used = Object.keys(customInstanceDbs.current ?? {})
		let n = 1
		while (used.includes(`dt${n}`)) n++
		return `dt${n}`
	}

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
			// Rows already configured in an earlier pass keep their state; the wizard only
			// ever adds data tables, so a name that has left `missing` is done.
			const previous = new Map(rows.map((r) => [r.name, r]))
			rows = [...new Set(enabled.map((m) => m.datatable_name))].map((name) => {
				const prev = previous.get(name)
				if (prev && !missing.includes(name)) return { ...prev, status: 'done' as const }
				return {
					name,
					migrations: enabled.filter((m) => m.datatable_name === name),
					status: (missing.includes(name) ? 'unconfigured' : 'done') as Row['status'],
					justSaved: false
				}
			})
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
	/**
	 * The data table now exists — run the migrations that were skipped for it during the
	 * import, which is the whole reason this step waits for the configuration.
	 */
	async function runMigrationsFor(name: string): Promise<void> {
		const row = rows.find((r) => r.name === name)
		if (!row) return
		working = true
		row.status = 'running'
		try {
			for (const m of row.migrations) await applyOneMigration(workspace, slug, m)
			row.status = 'done'
			row.error = undefined
			// One-shot, cleared by name rather than by reference: `load()` rebuilds the row
			// objects, so the one holding the flag when it fires may not be this one.
			row.justSaved = true
			setTimeout(() => {
				const current = rows.find((r) => r.name === name)
				if (current) current.justSaved = false
			}, 1500)
		} catch (e: any) {
			row.status = 'failed'
			row.error = e?.body ?? e?.message ?? String(e)
			sendUserToast(`Could not run the migrations for ${name}: ${row.error}`, true)
		} finally {
			working = false
		}
	}

	/**
	 * After the wizard closes. The migrations already ran inside its checklist, via
	 * `onFinishAlso`, so this only re-reads what exists now — including the case where
	 * the wizard was cancelled, or made a table under a different name than the row
	 * asked for, which leaves the row outstanding rather than falsely done.
	 */
	async function afterWizard(): Promise<void> {
		const name = wizardFor
		wizardFor = undefined
		try {
			const tables = await WorkspaceService.listDataTables({ workspace })
			configuredNames = tables.map((t) => ({ name: t.name, resourcePath: t.resource_path }))
			const present = new Set(tables.map((t) => t.name))
			const row = name ? rows.find((r) => r.name === name) : undefined
			if (row && row.status !== 'done' && row.status !== 'failed' && !present.has(name!)) {
				row.status = 'unconfigured'
			}
		} catch {
			// Nothing to correct with; the row keeps whatever the run left it saying.
		}
	}
</script>

<div class="flex flex-col gap-4">
	<div>
		<h2 class="text-sm font-semibold text-emphasis">Finish setting up</h2>
		<!-- Reads as what the user gets out of it, not as what the import failed to do:
		     the step is skippable, so it has to say why finishing is worth their time. -->
		<p class="mt-0.5 text-xs text-secondary">
			Your project is imported. For its apps and flows to actually run, they need a place to store
			data and credentials for the services they use — the import can't supply those for you.
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
		{#if rows.length > 0}
			<!-- Named and explained: the row underneath is a table called `main`, which
			     says nothing to someone meeting the concept for the first time. -->
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">
					Data table{rows.length === 1 ? '' : 's'} to set up ({rows.length})
				</span>
				<p class="text-xs font-normal text-secondary">
					Where apps and flows keep the data they read and write.
				</p>
			</div>
		{/if}
		<ul class="flex flex-col gap-1.5">
			{#each rows as row (row.name)}
				<ImportSetupRow flash={row.justSaved}>
					{#snippet icon()}
						{#if row.status === 'done'}
							<Check size={20} class="text-emerald-600" />
						{:else if row.status === 'running'}
							<Loader2 size={20} class="animate-spin text-blue-500" />
						{:else if row.status === 'failed'}
							<X size={20} class="text-red-500" />
						{:else}
							<Database size={20} class="text-secondary" />
						{/if}
					{/snippet}
					{#snippet title()}
						<span class="min-w-0 truncate font-mono text-emphasis">{row.name}</span>
					{/snippet}
					{#snippet detail()}
						<span class="truncate text-secondary">
							{#if row.status === 'done'}
								{row.migrations.length} migration{row.migrations.length === 1 ? '' : 's'} run
							{:else if row.status === 'running'}
								running migrations…
							{:else if row.status === 'failed'}
								<span class="text-red-500">{row.error}</span>
							{:else}
								not configured yet
							{/if}
						</span>
					{/snippet}
					{#snippet action()}
						<!-- The wizard owns creating a data table: picking or provisioning the
						     database, writing the config, and reporting the connection. This step
						     only says which name it needs and runs the migrations afterwards. -->
						<Button
							variant={row.status === 'done' ? 'subtle' : 'accent'}
							unifiedSize="sm"
							disabled={working}
							onClick={() => openWizard(row.name)}
						>
							{row.status === 'done' ? 'Configured' : 'Set up'}
						</Button>
					{/snippet}
				</ImportSetupRow>
			{/each}
		</ul>

		{#if blanks.length > 0}
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-emphasis"
					>Credentials to fill ({blanks.length})</span
				>
				<ul class="flex flex-col gap-1.5">
					{#each blanks as b (b.path)}
						{@const canConnect = !b.done && connectable.has(b.resourceType)}
						<!-- Laid out like the resource type rows in the Add-a-resource drawer: the
						     integration's own icon, its product name, and the raw identifier demoted
						     beside it. The path only matters when two resources share a type, so it
						     stops being the thing the eye lands on. -->
						<ImportSetupRow flash={b.justSaved}>
							{#snippet icon()}
								{#if b.done}
									<Check size={20} class="text-emerald-600" />
								{:else}
									<IconedResourceType name={b.resourceType} silent width="20px" height="20px" />
								{/if}
							{/snippet}
							{#snippet title()}
								<div class="flex min-w-0 flex-row items-baseline gap-2">
									<span class="min-w-0 truncate text-emphasis">
										{resourceTypeDisplayName(b.resourceType)}
									</span>
									<span class="min-w-0 truncate font-mono text-2xs font-normal text-hint">
										{b.path}
									</span>
								</div>
							{/snippet}
							{#snippet detail()}
								{#if !b.done && b.missing.length > 0}
									<span class="truncate text-secondary">
										Missing {b.missing.join(', ')}
									</span>
								{/if}
							{/snippet}
							{#snippet action()}
								<!-- Connect where the instance has a client for this type: asking for an
								     OAuth resource by hand means pasting an access token that dies within
								     the hour, since only a token Windmill obtained itself gets refreshed. -->
								<Button
									variant={b.done ? 'subtle' : 'accent'}
									unifiedSize="sm"
									disabled={working}
									onClick={() =>
										canConnect
											? appConnect?.open(b.resourceType, b.path)
											: resourceEditor?.initEdit(b.path)}
								>
									{b.done ? 'Saved' : canConnect ? 'Connect' : 'Fill in'}
								</Button>
							{/snippet}
						</ImportSetupRow>
					{/each}
				</ul>
			</div>
		{/if}

		<!-- The same slot says two different things. While work is outstanding: info, not
		     warning — skipping is a supported choice with a consequence, not a problem the
		     user caused, and the rows above already carry the urgency. Once nothing is
		     outstanding it has no caveat left to give, so it confirms instead. -->
		{#if outstanding === 0}
			<Alert type="success" title="You're all set" size="xs">
				Everything this project needs is configured. Finish, and it is ready to run.
			</Alert>
		{:else}
			<Alert type="info" title="You can skip this" size="xs" collapsible>
				The project's apps and flows will fail wherever they read something that is still
				unconfigured. Everything else it imported works either way, and you can come back to this
				from the workspace at any time.
			</Alert>
		{/if}
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
			<!-- Every row carries its own action, so the footer only offers the way out —
			     twice, because leaving work undone is a different decision from having
			     finished it. Finish stays disabled until nothing is outstanding, and Skip
			     is the subtle escape beside it. A load that failed cannot tell what is
			     outstanding, so it offers Finish rather than blocking on an unknown. -->
			{#if outstanding > 0 && !loading && !loadError}
				<Button variant="subtle" unifiedSize="sm" disabled={working} onClick={onSkip}>
					Skip for now
				</Button>
			{/if}
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={working || (outstanding > 0 && !loadError)}
				onClick={onFinish}
			>
				Finish setup →
			</Button>
		</div>
	</div>
</div>

{#if wizardOpen || wizardFor}
	<AddDataTableWizard
		bind:this={wizard}
		bind:opened={wizardOpen}
		initialName={wizardFor}
		modalTarget="body"
		finishAlso="run migrations"
		onFinishAlso={() => runMigrationsFor(wizardFor ?? '')}
		existingNames={configuredNames.map((c) => c.name)}
		existingDataTables={configuredNames}
		onDone={() => void afterWizard()}
		{customInstanceDbs}
		{confirmationModal}
		{defaultInstanceDbName}
	/>
{/if}
<ConfirmationModal {...confirmationModal.props} />

<!-- The destination is not the workspace the app is in until the run switches to it,
     so the editor is told which one explicitly.

     Saving re-reads only the resources, never `load()`: a credential cannot change which
     data tables the project ships or which ones the workspace has, and `load()` raises
     `loading`, which replaces both lists with the spinner — so every save looked like the
     whole step had reloaded. -->
<ResourceEditorDrawer
	bind:this={resourceEditor}
	{workspace}
	onSaved={() => void refreshBlanks()}
	onRestored={() => void refreshBlanks()}
/>

<!-- `on:refresh` fires once the connection has been written into the stub — the same moment
     a save is — so the rows settle the same way either route was taken. -->
<AppConnectDrawer bind:this={appConnect} {workspace} on:refresh={() => void refreshBlanks()} />

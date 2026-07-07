<script lang="ts">
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'
	import { workspaceStore, enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		ScriptService,
		FlowService,
		AppService,
		ResourceService,
		ScheduleService,
		FolderService,
		WorkspaceService,
		HttpTriggerService,
		WebsocketTriggerService,
		KafkaTriggerService,
		NatsTriggerService,
		MqttTriggerService,
		SqsTriggerService,
		GcpTriggerService,
		AzureTriggerService,
		PostgresTriggerService,
		EmailTriggerService
	} from '$lib/gen'
	import FolderPicker from '$lib/components/FolderPicker.svelte'
	import {
		rewriteAppValue,
		rewriteContent,
		rewriteFlowValue,
		rewriteRawAppContent
	} from '$lib/components/workspaceSettings/projectBundle'
	import MigrationSqlEditor from '$lib/components/workspaceSettings/MigrationSqlEditor.svelte'
	import { runScriptAndPollResult } from '$lib/components/jobs/utils'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { Cloud, Download, Loader2 } from 'lucide-svelte'

	type ExportItem = Record<string, any>
	interface ProjectMigration {
		datatable_name: string
		sql: string
		sql_down?: string
		enabled: boolean
	}
	interface ProjectExport {
		project: { slug: string; name: string; summary: string; readme: string | null }
		scripts: ExportItem[]
		flows: ExportItem[]
		apps: ExportItem[]
		resources: ExportItem[]
		triggers: ExportItem[]
		migrations?: ProjectMigration[]
	}

	let slug = $derived($page.url.searchParams.get('hub') ?? '')
	let workspace = $derived($workspaceStore)

	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let data = $state<ProjectExport | undefined>(undefined)
	let installing = $state(false)
	// True while the migration review/missing-datatable modals are open, before the
	// import spinner starts — keeps the Import button from launching a second import.
	let planningMigrations = $state(false)
	let results = $state<{ path: string; ok: boolean; error?: string }[]>([])
	let done = $state(false)
	let folderName = $state('')

	// When the target lacks a needed data table, "import without that migration".
	const missingDatatableModal = createAsyncConfirmationModal()

	// Migration review drawer: preview + edit each runnable migration's SQL and
	// choose which to run, resolved linearly via `reviewResolve`.
	let reviewDrawer = $state<Drawer | undefined>()
	let reviewList = $state<
		{ datatable_name: string; sql: string; sql_down: string; run: boolean }[]
	>([])
	// Bumped per review session so the Monaco editors re-mount with the new SQL.
	let reviewGeneration = $state(0)
	let reviewResolve: ((run: boolean) => void) | undefined
	function openMigrationReview(migs: ProjectMigration[]): Promise<boolean> {
		reviewList = migs.map((m) => ({
			datatable_name: m.datatable_name,
			sql: m.sql,
			sql_down: m.sql_down ?? '',
			run: true
		}))
		reviewGeneration++
		reviewDrawer?.openDrawer()
		return new Promise((resolve) => (reviewResolve = resolve))
	}
	function closeMigrationReview(run: boolean) {
		// Capture + clear first so the `on:close` fired by closeDrawer() (which would
		// call this again with run=false) can't override an explicit Run/Skip choice.
		const resolve = reviewResolve
		reviewResolve = undefined
		reviewDrawer?.closeDrawer()
		resolve?.(run)
	}

	let loadSeq = 0

	$effect(() => {
		if (slug && workspace) void load()
	})

	async function load() {
		// Token + captured slug/workspace: a slow /export for an old ?hub= must not
		// overwrite the data of a newer one once we've navigated away.
		const reqSeq = ++loadSeq
		const reqSlug = slug
		const reqWorkspace = workspace
		loading = true
		loadError = undefined
		try {
			const res = await fetch(
				`/api/w/${reqWorkspace}/hub/projects/${encodeURIComponent(reqSlug)}/export`,
				{ credentials: 'include', headers: { accept: 'application/json' } }
			)
			const text = await res.text()
			if (reqSeq !== loadSeq) return // a newer load() superseded this one
			if (!res.ok) throw new Error(`export ${res.status}: ${text}`)
			data = JSON.parse(text)
			if (data && !folderName) folderName = data.project.slug
		} catch (e: any) {
			if (reqSeq !== loadSeq) return
			loadError = e?.message ?? String(e)
		} finally {
			if (reqSeq === loadSeq) loading = false
		}
	}

	const counts = $derived(
		data
			? {
					scripts: data.scripts.length,
					flows: data.flows.length,
					apps: data.apps.length,
					resources: data.resources.length,
					triggers: data.triggers.length,
					migrations: (data.migrations ?? []).filter(
						(m) => m.enabled && (m.sql ?? '').trim() !== ''
					).length
				}
			: undefined
	)

	// Surface the backend's explanation: API errors carry the real message in
	// `.body` (plain text for Windmill 4xx), while `.message` is the generic
	// status text ("Bad Request"). Prefer the body so e.g. a path/route_path
	// collision reads as the actual reason, not just "Bad Request".
	function errorMessage(e: any): string {
		const body = e?.body
		if (typeof body === 'string' && body.trim() !== '') return body
		if (body && typeof body === 'object')
			return body.error?.message ?? body.message ?? JSON.stringify(body)
		return e?.message ?? String(e)
	}

	function record(path: string, p: Promise<unknown>): Promise<void> {
		return p.then(
			() => {
				results = [...results, { path, ok: true }]
			},
			(e: any) => {
				results = [...results, { path, ok: false, error: errorMessage(e) }]
			}
		)
	}

	// Minimal non-public policy for re-created apps.
	const defaultPolicy = { execution_mode: 'publisher', triggerables_v2: {} } as any

	// EE-only kinds; the rest (http, websocket, postgres, mqtt, email) work on CE.
	const EE_TRIGGER_KINDS = new Set(['kafka', 'nats', 'sqs', 'gcp', 'azure'])

	// Non-schedule trigger creators, keyed by kind (schedule has its own body shape).
	const TRIGGER_CREATE: Record<string, (workspace: string, requestBody: any) => Promise<unknown>> =
		{
			http: (workspace, requestBody) =>
				HttpTriggerService.createHttpTrigger({ workspace, requestBody }),
			websocket: (workspace, requestBody) =>
				WebsocketTriggerService.createWebsocketTrigger({ workspace, requestBody }),
			kafka: (workspace, requestBody) =>
				KafkaTriggerService.createKafkaTrigger({ workspace, requestBody }),
			nats: (workspace, requestBody) =>
				NatsTriggerService.createNatsTrigger({ workspace, requestBody }),
			mqtt: (workspace, requestBody) =>
				MqttTriggerService.createMqttTrigger({ workspace, requestBody }),
			sqs: (workspace, requestBody) =>
				SqsTriggerService.createSqsTrigger({ workspace, requestBody }),
			gcp: (workspace, requestBody) =>
				GcpTriggerService.createGcpTrigger({ workspace, requestBody }),
			azure: (workspace, requestBody) =>
				AzureTriggerService.createAzureTrigger({ workspace, requestBody }),
			postgres: (workspace, requestBody) =>
				PostgresTriggerService.createPostgresTrigger({ workspace, requestBody }),
			email: (workspace, requestBody) =>
				EmailTriggerService.createEmailTrigger({ workspace, requestBody })
		}

	// Map bundled paths `f/<fromSlug>/...` -> `f/<folder>/...`. Only enumerated
	// paths go in, so rewriters touch real refs, never incidental text.
	function buildRetargetMap(
		bundle: ProjectExport,
		fromSlug: string,
		folder: string
	): Map<string, string> {
		const map = new Map<string, string>()
		const prefix = `f/${fromSlug}/`
		const add = (p: unknown) => {
			if (typeof p === 'string' && p.startsWith(prefix)) {
				map.set(p, `f/${folder}/${p.slice(prefix.length)}`)
			}
		}
		for (const s of bundle.scripts) add(s.path)
		for (const f of bundle.flows) add(f.path)
		for (const a of bundle.apps) add(a.path)
		for (const r of bundle.resources) add(r.path)
		for (const t of bundle.triggers) {
			add(t.path)
			add(t.runnable_path)
		}
		return map
	}

	// Structural retarget: rewrite each item's path and its internal refs,
	// leaving Hub refs and arbitrary content untouched.
	function retarget(bundle: ProjectExport, fromSlug: string, folder: string): ProjectExport {
		if (folder === fromSlug) return bundle
		const map = buildRetargetMap(bundle, fromSlug, folder)
		const remap = (p: unknown) => (typeof p === 'string' ? (map.get(p) ?? p) : p)
		return {
			...bundle,
			scripts: bundle.scripts.map((s) => ({
				...s,
				path: remap(s.path),
				content: rewriteContent(s.content ?? '', map)
			})),
			flows: bundle.flows.map((f) => ({
				...f,
				path: remap(f.path),
				value: rewriteFlowValue(f.value, map)
			})),
			apps: bundle.apps.map((a) => ({
				...a,
				path: remap(a.path),
				// Raw apps keep their structure in the `value.raw` JSON string.
				value:
					a.app_type === 'raw'
						? { ...a.value, raw: rewriteRawAppContent(a.value?.raw ?? '', map) }
						: rewriteAppValue(a.value, map)
			})),
			resources: bundle.resources.map((r) => ({ ...r, path: remap(r.path) })),
			triggers: bundle.triggers.map((t) => ({
				...t,
				path: remap(t.path),
				runnable_path: remap(t.runnable_path),
				// `$res:` refs can live in trigger args/config.
				config: t.config ? JSON.parse(rewriteContent(JSON.stringify(t.config), map)) : t.config
			}))
		}
	}

	// Decide which data table migrations to run. Migrations are keyed by data table
	// name and applied only to a target data table of the same name. Returns the
	// migrations to run (with any edits the user made), an empty array when there's
	// nothing to run, or `null` when the user backs out of the whole import at the
	// missing-data-table warning.
	async function planMigrations(
		workspace: string,
		migrations: ProjectMigration[]
	): Promise<ProjectMigration[] | null> {
		const enabled = migrations.filter((m) => m.enabled && (m.sql ?? '').trim() !== '')
		if (enabled.length === 0) return []

		let present: Set<string>
		try {
			const dts = await WorkspaceService.listDataTables({ workspace })
			present = new Set(dts.map((d) => d.name))
		} catch {
			// Can't read the target's data tables — skip migrations rather than guess.
			return []
		}
		const runnable = enabled.filter((m) => present.has(m.datatable_name))
		const missingNames = [
			...new Set(enabled.filter((m) => !present.has(m.datatable_name)).map((m) => m.datatable_name))
		]

		// Warn about missing data tables first: confirming imports without their
		// migrations, cancelling backs out of the whole import so the user can create
		// the data table(s) and re-run.
		if (missingNames.length > 0) {
			const proceed = await missingDatatableModal.ask({
				title: 'Some data tables are missing',
				confirmationText: 'Import without them',
				children: `This project uses data table(s) "${missingNames.join(
					'", "'
				)}" that don't exist in this workspace, so their migrations will be skipped. To apply them, cancel, create the data table(s) with the same name in Workspace settings → Data tables, then re-run this import.`
			})
			if (!proceed) return null
		}

		let toRun: ProjectMigration[] = []
		if (runnable.length > 0) {
			const run = await openMigrationReview(runnable)
			if (run) {
				toRun = reviewList
					.filter((r) => r.run && r.sql.trim() !== '')
					.map((r) => ({
						datatable_name: r.datatable_name,
						sql: r.sql,
						sql_down: r.sql_down,
						enabled: true
					}))
			}
		}
		return toRun
	}

	// Apply one migration to the target data table. If the data table opted into
	// migrations, record it (datatable_migrations + _wm_migrations, run only this
	// version); otherwise run the SQL once as a preview job (unrecorded).
	async function applyOneMigration(workspace: string, m: ProjectMigration): Promise<void> {
		let recorded = false
		try {
			const status = await WorkspaceService.getDatatableMigrationsStatus({
				workspace,
				datatableName: m.datatable_name
			})
			recorded = !!status.enabled
		} catch {}

		if (recorded) {
			// Record the shipped down migration (DROP the created tables) so it can be
			// rolled back.
			const codeDown = (m.sql_down ?? '').trim()
			const created = await WorkspaceService.createDatatableMigration({
				workspace,
				datatableName: m.datatable_name,
				requestBody: {
					name: `hub_import_${data?.project.slug ?? 'project'}`,
					code_up: m.sql,
					code_down: codeDown || undefined
				}
			})
			await WorkspaceService.runDatatableMigrations({
				workspace,
				datatableName: m.datatable_name,
				only: created.timestamp
			})
		} else {
			await runScriptAndPollResult({
				workspace,
				requestBody: {
					language: 'postgresql',
					content: m.sql,
					args: { database: `datatable://${m.datatable_name}` }
				}
			})
		}
	}

	async function install() {
		// Snapshot reactive state up-front: `workspace` ($derived) and `data`
		// ($state, replaced by load()) can both change mid-import on a workspace
		// switch, which would split items or mix two exports. Pin both.
		// Guard against a second click while the review modal is open (the Import
		// button isn't `installing` yet during planning, so it would otherwise be
		// clickable and start a concurrent import).
		if (installing || planningMigrations) return
		const workspace = $workspaceStore
		const exportData = data
		if (!exportData || !workspace) return
		const folder = folderName.trim() || exportData.project.slug

		// Review data table migrations first (before the import spinner), so the user
		// previews/edits and decides, then the whole import runs uninterrupted.
		planningMigrations = true
		let migrationsToRun: ProjectMigration[] | null
		try {
			migrationsToRun = await planMigrations(workspace, exportData.migrations ?? [])
		} finally {
			planningMigrations = false
		}
		// User backed out at the missing-data-table warning — abort the whole import.
		if (migrationsToRun === null) return

		installing = true
		results = []
		done = false
		try {
			try {
				await FolderService.createFolder({ workspace, requestBody: { name: folder } })
			} catch {}

			const proj = retarget(exportData, exportData.project.slug, folder)
			for (const s of proj.scripts) {
				await record(
					s.path,
					ScriptService.createScript({
						workspace,
						requestBody: {
							path: s.path,
							summary: s.summary ?? '',
							description: s.description ?? '',
							content: s.content ?? '',
							language: s.language,
							schema: s.schema ?? undefined,
							kind: s.kind ?? 'script',
							lock: s.lockfile ?? undefined
						}
					})
				)
			}
			for (const f of proj.flows) {
				await record(
					f.path,
					FlowService.createFlow({
						workspace,
						requestBody: {
							path: f.path,
							summary: f.summary ?? '',
							description: f.description ?? '',
							value: f.value,
							schema: f.schema ?? undefined
						}
					})
				)
			}
			for (const r of proj.resources) {
				await record(
					r.path,
					ResourceService.createResource({
						workspace,
						updateIfExists: false,
						requestBody: {
							path: r.path,
							resource_type: r.resource_type,
							value: {},
							description: 'Imported stub — fill in the value.'
						}
					})
				)
			}
			for (const a of proj.apps) {
				if (a.app_type === 'raw') {
					await record(
						a.path,
						(async () => {
							let parsed: any
							try {
								parsed = JSON.parse(a.value?.raw ?? '{}')
							} catch (e: any) {
								throw new Error(`invalid raw app bundle: ${e?.message ?? String(e)}`)
							}
							const files = { ...(parsed.files ?? {}) }
							const js = files['/bundle.js'] ?? ''
							const css = files['/bundle.css'] ?? ''
							delete files['/bundle.js']
							delete files['/bundle.css']
							return AppService.createAppRaw({
								workspace,
								formData: {
									app: {
										path: a.path,
										summary: a.summary ?? '',
										value: { files, runnables: parsed.runnables ?? {} },
										policy: defaultPolicy
									},
									js,
									css
								}
							})
						})()
					)
				} else {
					await record(
						a.path,
						AppService.createApp({
							workspace,
							requestBody: {
								path: a.path,
								summary: a.summary ?? '',
								value: a.value,
								policy: defaultPolicy
							}
						})
					)
				}
			}
			for (const t of proj.triggers) {
				if (t.kind === 'schedule') {
					await record(
						t.path,
						ScheduleService.createSchedule({
							workspace,
							requestBody: {
								path: t.path,
								schedule: t.config?.schedule ?? '0 0 * * * *',
								timezone: t.config?.timezone ?? 'UTC',
								script_path: t.runnable_path,
								is_flow: t.runnable_kind === 'flow',
								enabled: false,
								args: t.config?.args ?? {},
								summary: t.summary ?? null
							}
						})
					)
				} else {
					const create = TRIGGER_CREATE[t.kind]
					if (!create) {
						await record(
							t.path,
							Promise.reject(new Error(`trigger kind '${t.kind}' not supported yet`))
						)
					} else if (EE_TRIGGER_KINDS.has(t.kind) && !$enterpriseLicense) {
						await record(
							t.path,
							Promise.reject(new Error(`trigger kind '${t.kind}' requires Enterprise`))
						)
					} else {
						// `config` holds only kind-specific fields; explicit fields win.
						// `mode: 'disabled'` imports it disabled (non-schedule uses `mode`).
						const requestBody = {
							...(t.config ?? {}),
							path: t.path,
							script_path: t.runnable_path,
							is_flow: t.runnable_kind === 'flow',
							summary: t.summary ?? null,
							mode: 'disabled'
						}
						await record(t.path, create(workspace, requestBody))
					}
				}
			}

			// Apply the reviewed data table migrations after items exist. Each is
			// recorded (or run as a preview job) per applyOneMigration.
			for (const m of migrationsToRun) {
				await record(`data table: ${m.datatable_name}`, applyOneMigration(workspace, m))
			}

			done = true
			const failed = results.filter((r) => !r.ok).length
			sendUserToast(
				failed > 0
					? `Imported with ${failed} item(s) failed.`
					: `Project imported into ${workspace}.`,
				failed > 0
			)
		} finally {
			installing = false
		}
	}
</script>

<div class="mx-auto w-full max-w-screen-md px-4 py-10">
	{#if !slug}
		<p class="text-sm text-secondary">Missing <span class="font-mono">?hub=&lt;slug&gt;</span>.</p>
	{:else if loading}
		<div class="flex items-center gap-2 text-sm text-secondary">
			<Loader2 size={16} class="animate-spin" /> Loading project…
		</div>
	{:else if loadError}
		<p class="text-sm text-red-600">Failed to load project: {loadError}</p>
	{:else if data}
		<h1 class="text-2xl font-semibold text-primary">Add “{data.project.name}” to workspace</h1>
		<p class="mt-1 text-sm text-secondary">{data.project.summary}</p>

		<div class="mt-4 max-w-xs">
			<p class="mb-1 text-xs text-tertiary">
				Folder in <span class="font-mono">{workspace}</span>
			</p>
			<FolderPicker bind:folderName disabled={installing || done} size="sm" />
			<p class="mt-1 text-xs text-tertiary">
				Items import under <span class="font-mono">f/{folderName.trim() || data.project.slug}/</span
				>.
			</p>
		</div>

		<div class="mt-6 flex flex-wrap gap-2 text-xs">
			<span class="rounded border px-2 py-1">{counts?.scripts} scripts</span>
			<span class="rounded border px-2 py-1">{counts?.flows} flows</span>
			<span class="rounded border px-2 py-1">{counts?.apps} apps</span>
			<span class="rounded border px-2 py-1">{counts?.resources} resources</span>
			<span class="rounded border px-2 py-1">{counts?.triggers} triggers</span>
			{#if counts && counts.migrations > 0}
				<span class="rounded border px-2 py-1">{counts.migrations} data table migrations</span>
			{/if}
		</div>

		<div
			class="mt-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-xs text-amber-900 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-100"
		>
			Resources are imported as empty stubs — set their values after import; a resource whose path
			already exists is reported as failed (existing values are never overwritten). All trigger
			kinds are recreated disabled; Kafka, NATS, SQS, GCP and Azure triggers require Enterprise.
			Triggers that reference a resource depend on stubs imported empty, so fill in the resource
			value before re-enabling the trigger.
		</div>

		<div class="mt-6 flex items-center gap-3">
			<Button
				variant="accent"
				startIcon={{ icon: done ? Cloud : Download }}
				disabled={installing || done || planningMigrations}
				onclick={install}
			>
				{#if installing}
					<Loader2 size={16} class="animate-spin mr-1" /> Importing…
				{:else if done}
					Imported
				{:else}
					Import to {workspace}
				{/if}
			</Button>
			{#if done}
				<Button variant="border" onclick={() => goto(`/`)}>Go to workspace</Button>
			{/if}
		</div>

		{#if results.length}
			<ul class="mt-6 flex flex-col gap-1 text-xs">
				{#each results as r}
					<li class="flex items-center gap-2">
						<span class={r.ok ? 'text-emerald-600' : 'text-red-600'}>{r.ok ? '✓' : '✗'}</span>
						<span class="font-mono">{r.path}</span>
						{#if !r.ok}<span class="text-red-600">— {r.error}</span>{/if}
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</div>

<Portal>
	<ConfirmationModal {...missingDatatableModal.props} />
</Portal>

<Drawer bind:this={reviewDrawer} size="700px" on:close={() => closeMigrationReview(false)}>
	<DrawerContent title="Data table migrations" on:close={() => closeMigrationReview(false)}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				This project ships migrations that recreate the data tables it uses. Review and edit the
				SQL, then choose which to run. A migration runs against the data table of the same name in
				<span class="font-mono">{workspace}</span>; if that data table has migrations enabled it is
				recorded, otherwise it runs once as a preview job.
			</p>
			{#each reviewList as m (m.datatable_name)}
				<div class="flex flex-col gap-1.5 rounded border bg-surface-secondary p-2 text-xs">
					<div class="flex items-center justify-between gap-2">
						<span class="font-mono text-primary">{m.datatable_name}</span>
						<Toggle bind:checked={m.run} size="xs" options={{ right: 'Run' }} />
					</div>
					{#if m.run}
						<MigrationSqlEditor
							bind:up={m.sql}
							bind:down={m.sql_down}
							generation={reviewGeneration}
						/>
					{/if}
				</div>
			{/each}
		</div>
		{#snippet actions()}
			<Button variant="border" onclick={() => closeMigrationReview(false)}>Skip migrations</Button>
			<Button
				variant="accent"
				disabled={!reviewList.some((m) => m.run && m.sql.trim() !== '')}
				onclick={() => closeMigrationReview(true)}
			>
				Run selected
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

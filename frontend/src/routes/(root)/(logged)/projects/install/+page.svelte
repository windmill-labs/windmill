<script lang="ts">
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'
	import { workspaceStore, enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import {
		ScriptService,
		FlowService,
		AppService,
		ResourceService,
		ScheduleService,
		FolderService,
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
	import { Cloud, Download, Loader2 } from 'lucide-svelte'

	type ExportItem = Record<string, any>
	interface ProjectExport {
		project: { slug: string; name: string; summary: string; readme: string | null }
		scripts: ExportItem[]
		flows: ExportItem[]
		apps: ExportItem[]
		resources: ExportItem[]
		triggers: ExportItem[]
	}

	let slug = $derived($page.url.searchParams.get('hub') ?? '')
	let workspace = $derived($workspaceStore)

	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let data = $state<ProjectExport | undefined>(undefined)
	let installing = $state(false)
	let results = $state<{ path: string; ok: boolean; error?: string }[]>([])
	let done = $state(false)
	let folderName = $state('')

	$effect(() => {
		if (slug && workspace) void load()
	})

	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await fetch(
				`/api/w/${workspace}/hub/projects/${encodeURIComponent(slug)}/export`,
				{ credentials: 'include', headers: { accept: 'application/json' } }
			)
			if (!res.ok) throw new Error(`export ${res.status}: ${await res.text()}`)
			data = JSON.parse(await res.text())
			if (data && !folderName) folderName = data.project.slug
		} catch (e: any) {
			loadError = e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	const counts = $derived(
		data
			? {
					scripts: data.scripts.length,
					flows: data.flows.length,
					apps: data.apps.length,
					resources: data.resources.length,
					triggers: data.triggers.length
				}
			: undefined
	)

	function record(path: string, p: Promise<unknown>): Promise<void> {
		return p.then(
			() => {
				results = [...results, { path, ok: true }]
			},
			(e: any) => {
				results = [...results, { path, ok: false, error: e?.message ?? String(e) }]
			}
		)
	}

	// Minimal non-public policy for re-created apps.
	const defaultPolicy = { execution_mode: 'publisher', triggerables_v2: {} } as any

	// Trigger kinds that require an Enterprise license. The others (http,
	// websocket, postgres, mqtt, email) are available on CE. `schedule` is
	// handled separately because it has a distinct request-body shape.
	const EE_TRIGGER_KINDS = new Set(['kafka', 'nats', 'sqs', 'gcp', 'azure'])

	// Non-schedule trigger creators, keyed by kind.
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

	async function install() {
		// Snapshot the target workspace once: the module-level `workspace` is
		// reactive ($derived), so a workspace switch mid-import would otherwise
		// split items across workspaces (folder in A, later items in B).
		const workspace = $workspaceStore
		if (!data || !workspace) return
		const folder = folderName.trim() || data.project.slug
		installing = true
		results = []
		done = false
		try {
			try {
				await FolderService.createFolder({ workspace, requestBody: { name: folder } })
			} catch {}

			const proj = retarget(data, data.project.slug, folder)
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
						// `config` carries only kind-specific fields (publish strips path/
						// script_path/is_flow/enabled/summary), so the explicit fields win.
						// `mode: 'disabled'` imports the trigger disabled (non-schedule
						// triggers use `mode`, not the deprecated `enabled` flag).
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
				disabled={installing || done}
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

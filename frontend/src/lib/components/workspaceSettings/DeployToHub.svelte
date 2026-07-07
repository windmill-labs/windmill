<script lang="ts">
	import { base } from '$lib/base'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import WorkspaceDeployLayout from '$lib/components/WorkspaceDeployLayout.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		AppService,
		FlowService,
		JobService,
		RawAppService,
		ResourceService,
		ScriptService,
		WorkspaceService,
		HttpTriggerService,
		WebsocketTriggerService,
		KafkaTriggerService,
		NatsTriggerService,
		SqsTriggerService,
		MqttTriggerService,
		GcpTriggerService,
		AzureTriggerService,
		PostgresTriggerService,
		EmailTriggerService,
		ScheduleService
	} from '$lib/gen'
	import { workspaceStore, hubBaseUrlStore, enterpriseLicense } from '$lib/stores'
	import { sleep, emptySchema, displayDate } from '$lib/utils'
	import { computeSecretUrl } from '$lib/components/apps/editor/appDeploy.svelte'
	import {
		buildProjectBundle,
		buildPathMap,
		extractScriptRefs,
		extractFlowRefs,
		extractAppRefs,
		type BundleDeps,
		type BundledItem,
		type FetchedItem,
		type ItemKind,
		type ItemRef,
		type ProjectBundle
	} from './projectBundle'
	import {
		detectDatatableTables,
		generateDatatableMigrations,
		type GeneratedMigration
	} from './projectMigrations'
	import Toggle from '../Toggle.svelte'
	import {
		Check,
		Cloud,
		Code2,
		Copy,
		Database,
		ExternalLink,
		Globe,
		Info,
		LayoutDashboard,
		Loader2,
		Play,
		RotateCcw,
		TriangleAlert,
		X,
		Zap
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import type { Kind } from '$lib/utils_deployable'

	type Phase = 'predeploy' | 'draft' | 'under_review' | 'live'
	type RecStatus = 'none' | 'recorded'
	interface DeployItem {
		key: string
		path: string
		kind: Kind
		summary?: string
		rec: RecStatus
		published?: boolean
		publicUrl?: string
		[k: string]: unknown
	}

	const canRecord = (k: Kind) => k === 'script' || k === 'flow'
	const canPublishApp = (k: Kind) => k === 'app' || k === 'raw_app'

	type TriggerKindLabel =
		| 'http'
		| 'websocket'
		| 'schedule'
		| 'kafka'
		| 'nats'
		| 'sqs'
		| 'mqtt'
		| 'gcp'
		| 'azure'
		| 'postgres'
		| 'email'
	interface WorkspaceTrigger {
		kind: TriggerKindLabel
		path: string
		script_path: string
		is_flow: boolean
		summary?: string
		config: Record<string, unknown>
	}
	// One row per trigger kind: badge label, workspace list route, optional
	// post-import note, and the config field holding its resource path.
	const TRIGGER_KINDS: Record<
		TriggerKindLabel,
		{ badge: string; route: string; note?: string; resourceField?: string }
	> = {
		http: {
			badge: 'HTTP',
			route: 'routes',
			note: 'Webhook URL regenerates on import — re-register with the external service.'
		},
		websocket: {
			badge: 'WebSocket',
			route: 'websocket_triggers',
			note: 'Reconnect WebSocket auth after import if external service requires it.'
		},
		schedule: { badge: 'Schedule', route: 'schedules' },
		kafka: {
			badge: 'Kafka',
			route: 'kafka_triggers',
			note: 'Verify Kafka broker access from the importing instance.',
			resourceField: 'kafka_resource_path'
		},
		nats: {
			badge: 'NATS',
			route: 'nats_triggers',
			note: 'Verify NATS connection from the importing instance.',
			resourceField: 'nats_resource_path'
		},
		sqs: { badge: 'SQS', route: 'sqs_triggers', resourceField: 'aws_resource_path' },
		mqtt: { badge: 'MQTT', route: 'mqtt_triggers', resourceField: 'mqtt_resource_path' },
		gcp: {
			badge: 'GCP Pub/Sub',
			route: 'gcp_triggers',
			note: 'Re-link GCP Pub/Sub subscription after import.',
			resourceField: 'gcp_resource_path'
		},
		azure: {
			badge: 'Azure',
			route: 'azure_triggers',
			note: 'Re-link Azure Event Grid subscription after import.',
			resourceField: 'azure_resource_path'
		},
		postgres: {
			badge: 'Postgres',
			route: 'postgres_triggers',
			resourceField: 'postgres_resource_path'
		},
		email: { badge: 'Email', route: 'email_triggers', note: 'Email address regenerates on import.' }
	}
	function triggerResourcePath(t: WorkspaceTrigger): string | undefined {
		const field = TRIGGER_KINDS[t.kind]?.resourceField
		const v = field ? (t.config as any)?.[field] : undefined
		return typeof v === 'string' && v !== '' ? v : undefined
	}

	// Folder name (no `f/`) this project is scoped to; provided by the /folders launcher.
	let { folder: folderProp }: { folder: string } = $props()

	let phase = $state<Phase>('predeploy')
	let workspaceItems = $state<DeployItem[]>([])
	let draftItems = $state<DeployItem[]>([])
	let workspaceTriggers = $state<WorkspaceTrigger[]>([])
	let triggersLoading = $state(false)
	let workspaceLoadSeq = 0
	// Per-call token: a license-late reload reuses workspaceLoadSeq, so only the
	// latest loadTriggers (highest token) may assign — a slow earlier one is dropped.
	let triggerLoadSeq = 0
	// Guards the reset+reload effect so a spurious same-value workspace-store emit
	// (e.g. a layout re-render) doesn't wipe state or close an open drawer.
	let lastLoadedKey = ''
	let schedulePreviews = $state<Record<string, string[]>>({})
	const schedulePreviewsInFlight = new Set<string>()

	// Project is always scoped to folderProp; selectedFolder is its `f/`-prefixed path.
	let selectedFolder = $derived(`f/${folderProp}`)
	function folderQs(folder: string = folderProp): string {
		return `?folder=${encodeURIComponent(folder)}`
	}
	let filteredWorkspaceItems = $derived(
		workspaceItems.filter((i) => i.path.startsWith(selectedFolder + '/'))
	)
	let items = $derived(phase === 'predeploy' ? filteredWorkspaceItems : draftItems)
	let manualDeselected = $state<Set<string>>(new Set())
	$effect(() => {
		selectedFolder
		phase
		manualDeselected = new Set()
	})
	let selectedItems = $derived(
		phase === 'predeploy' ? filteredWorkspaceItems.filter((i) => !manualDeselected.has(i.key)) : []
	)
	let selectedItemKeys = $derived(selectedItems.map((i) => i.key))
	let allSelected = $derived(
		phase === 'predeploy' && selectedItemKeys.length === filteredWorkspaceItems.length
	)
	function toggleItem(item: { key: string }) {
		const next = new Set(manualDeselected)
		if (next.has(item.key)) next.delete(item.key)
		else next.add(item.key)
		manualDeselected = next
	}
	function selectAll() {
		manualDeselected = new Set()
	}
	function deselectAll() {
		manualDeselected = new Set(filteredWorkspaceItems.map((i) => i.key))
	}
	let loading = $state(false)
	let workspaceRateLimit = $state<number | undefined>(undefined)
	let recordableItems = $derived(items.filter((i) => canRecord(i.kind)))
	let allRecorded = $derived(
		recordableItems.length > 0 && recordableItems.every((i) => i.rec === 'recorded')
	)

	let deploymentStatus = $state<
		Record<string, { status: 'loading' | 'deployed' | 'failed'; error?: string }>
	>({})
	let deploying = $state(false)

	let recordDrawer = $state<Drawer | undefined>()
	let recordTarget = $state<DeployItem | undefined>()
	let recordArgs = $state<Record<string, any>>({})
	let recordValid = $state(true)
	let recordSchema = $state<Record<string, any>>(emptySchema())
	let recordSchemaLoading = $state(false)
	type RunState = 'idle' | 'running' | 'success' | 'failed'
	let runState = $state<RunState>('idle')
	let runJobId = $state<string | undefined>(undefined)
	let runResult = $state<unknown>(undefined)
	let recordRunSeq = 0
	let runError = $state<string | undefined>(undefined)
	let recordings = $state<Record<string, string>>({})

	let publishDrawer = $state<Drawer | undefined>()
	let publishTarget = $state<DeployItem | undefined>()
	let publishing = $state(false)

	let resourceDrawer = $state<Drawer | undefined>()
	let triggerDrawer = $state<Drawer | undefined>()
	let bundleDrawer = $state<Drawer | undefined>()
	let hubName = $state('')
	let hubSummary = $state('')
	let hubReadme = $state('')

	let effectiveSlug = $state('')

	let hubSlug = $derived(effectiveSlug || sanitizeSlug(hubName))
	let hubUrl = $derived(`${$hubBaseUrlStore.replace(/\/+$/, '')}/projects/${hubSlug}`)

	function patchItem(key: string, patch: Partial<DeployItem>) {
		workspaceItems = workspaceItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
		draftItems = draftItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
	}

	async function listAllPages<T>(
		fetcher: (params: { perPage: number; page: number }) => Promise<T[]>
	): Promise<T[]> {
		const perPage = 100
		const out: T[] = []
		for (let page = 1; page <= 1000; page++) {
			const batch = await fetcher({ perPage, page })
			out.push(...batch)
			if (batch.length < perPage) return out
		}
		return out
	}

	async function loadWorkspace(workspace: string, seq: number) {
		loading = true
		try {
			const [apps, rawApps, flows, scripts, settings] = await Promise.all([
				listAllPages((p) => AppService.listApps({ workspace, ...p })),
				listAllPages((p) => RawAppService.listRawApps({ workspace, ...p })),
				listAllPages((p) => FlowService.listFlows({ workspace, ...p })),
				listAllPages((p) => ScriptService.listScripts({ workspace, ...p })),
				WorkspaceService.getSettings({ workspace }).catch(() => undefined)
			])
			if (seq !== workspaceLoadSeq) return

			workspaceRateLimit = settings?.public_app_execution_limit_per_minute

			const next: DeployItem[] = []
			const publicApps = apps.filter((a) => a.execution_mode === 'anonymous')
			const publicUrls = await Promise.all(
				publicApps.map((a) => resolvePublicUrl(workspace, a.path))
			)
			const publicUrlByPath = new Map(publicApps.map((a, i) => [a.path, publicUrls[i]]))
			for (const a of apps) {
				const isPublic = a.execution_mode === 'anonymous'
				// Raw apps live in the `app` table (value = files/runnables) but must be
				// published to the Hub as raw apps, not low-code apps.
				const isRaw = (a as any).raw_app === true
				next.push({
					key: `${isRaw ? 'raw_app' : 'app'}:${a.path}`,
					path: a.path,
					kind: isRaw ? 'raw_app' : 'app',
					appTable: isRaw || undefined,
					summary: a.summary,
					rec: 'none',
					published: isPublic,
					publicUrl: isPublic ? publicUrlByPath.get(a.path) : undefined
				})
			}
			for (const a of rawApps) {
				next.push({
					key: `raw_app:${a.path}`,
					path: a.path,
					kind: 'raw_app',
					summary: a.summary,
					rec: 'none'
				})
			}
			for (const f of flows) {
				next.push({
					key: `flow:${f.path}`,
					path: f.path,
					kind: 'flow',
					summary: f.summary,
					rec: 'none'
				})
			}
			for (const s of scripts) {
				next.push({
					key: `script:${s.path}`,
					path: s.path,
					kind: 'script',
					summary: s.summary,
					rec: 'none'
				})
			}
			if (seq !== workspaceLoadSeq) return
			workspaceItems = next
		} catch (e: any) {
			if (seq === workspaceLoadSeq) {
				sendUserToast(`Failed to load project items: ${e?.message ?? e}`, true)
			}
		} finally {
			if (seq === workspaceLoadSeq) loading = false
		}
	}

	async function loadTriggers(workspace: string, seq: number) {
		const tok = ++triggerLoadSeq
		triggersLoading = true
		try {
			// Kafka/NATS/SQS/GCP/Azure are EE-only (404 on CE) — skip without a license.
			// http/websocket/schedule/postgres/mqtt/email exist on CE.
			const safeList = async <T,>(p: Promise<T[]>): Promise<T[]> => {
				try {
					return await p
				} catch {
					return []
				}
			}
			const ee = !!$enterpriseLicense
			const eeList = <T,>(p: () => Promise<T[]>): Promise<T[]> =>
				ee ? safeList(p()) : Promise.resolve([])
			const [http, websocket, schedule, kafka, nats, sqs, mqtt, gcp, azure, postgres, email] =
				await Promise.all([
					safeList(HttpTriggerService.listHttpTriggers({ workspace })),
					safeList(WebsocketTriggerService.listWebsocketTriggers({ workspace })),
					safeList(ScheduleService.listSchedules({ workspace })),
					eeList(() => KafkaTriggerService.listKafkaTriggers({ workspace })),
					eeList(() => NatsTriggerService.listNatsTriggers({ workspace })),
					eeList(() => SqsTriggerService.listSqsTriggers({ workspace })),
					safeList(MqttTriggerService.listMqttTriggers({ workspace })),
					eeList(() => GcpTriggerService.listGcpTriggers({ workspace })),
					eeList(() => AzureTriggerService.listAzureTriggers({ workspace })),
					safeList(PostgresTriggerService.listPostgresTriggers({ workspace })),
					safeList(EmailTriggerService.listEmailTriggers({ workspace }))
				])
			if (seq !== workspaceLoadSeq || tok !== triggerLoadSeq) return
			const normalize = (
				kind: TriggerKindLabel,
				rows: Array<Record<string, any>>
			): WorkspaceTrigger[] =>
				rows.map((r) => ({
					kind,
					path: r.path,
					script_path: r.script_path,
					is_flow: r.is_flow ?? false,
					summary: r.summary,
					config: r
				}))
			workspaceTriggers = [
				...normalize('http', http),
				...normalize('websocket', websocket),
				...normalize('schedule', schedule),
				...normalize('kafka', kafka),
				...normalize('nats', nats),
				...normalize('sqs', sqs),
				...normalize('mqtt', mqtt),
				...normalize('gcp', gcp),
				...normalize('azure', azure),
				...normalize('postgres', postgres),
				...normalize('email', email)
			]
		} finally {
			if (seq === workspaceLoadSeq && tok === triggerLoadSeq) triggersLoading = false
		}
	}

	async function resolvePublicUrl(workspace: string, path: string): Promise<string | undefined> {
		try {
			const secret = await AppService.getPublicSecretOfApp({ workspace, path })
			return computeSecretUrl(secret)
		} catch {
			return undefined
		}
	}

	async function rehydrateFromHub(workspace: string, folder: string, seq: number) {
		try {
			const res = await fetch(`/api/w/${workspace}/hub/project${folderQs(folder)}`, {
				credentials: 'include',
				headers: { accept: 'application/json' }
			})
			if (seq !== workspaceLoadSeq) return
			if (!res.ok) return // 404 = no project published for this folder yet
			const p = JSON.parse(await res.text())
			if (seq !== workspaceLoadSeq || !p?.slug) return
			effectiveSlug = p.slug
			hubName = p.name ?? ''
			hubSummary = p.summary ?? ''
			hubReadme = p.readme ?? ''
			phase = p.status === 'live' ? 'live' : p.status === 'under_review' ? 'under_review' : 'draft'
			const ids: Record<string, number> = {}
			draftItems = (p.items ?? []).map((it: any) => {
				const wpath = it.source_path ?? it.path
				const key = `${it.kind}:${wpath}`
				if (typeof it.hub_id === 'number') ids[key] = it.hub_id
				return {
					key,
					path: wpath,
					kind: it.kind as Kind,
					summary: it.summary ?? undefined,
					rec: it.has_recording ? 'recorded' : 'none'
				} satisfies DeployItem
			})
			hubItemIds = ids
		} catch {}
	}

	$effect(() => {
		const ws = $workspaceStore
		const key = `${ws ?? ''}|${folderProp}`
		// Only reset+reload when the workspace or folder actually changes — not on
		// every store emit, which would close the record/publish drawer mid-use.
		if (ws && key !== lastLoadedKey) {
			lastLoadedKey = key
			const seq = ++workspaceLoadSeq
			// Wipe stale state before the parallel fetches resolve.
			workspaceItems = []
			workspaceTriggers = []
			schedulePreviews = {}
			manualDeselected = new Set()
			phase = 'predeploy'
			draftItems = []
			recordings = {}
			deploymentStatus = {}
			hubItemIds = {}
			effectiveSlug = ''
			hubName = ''
			hubSummary = ''
			hubReadme = ''
			previewItemCache.clear()
			previewTypeCache.clear()
			// Close any open drawers and clear their targets so stale item paths
			// from the previous workspace don't linger.
			recordDrawer?.closeDrawer()
			publishDrawer?.closeDrawer()
			recordTarget = undefined
			publishTarget = undefined
			loadWorkspace($workspaceStore, seq)
			loadTriggers($workspaceStore, seq)
			// Resume the folder's project if one exists.
			rehydrateFromHub($workspaceStore, folderProp, seq)
		}
	})

	// The EE license hydrates async; if it lands after a license-less loadTriggers,
	// EE kinds stay empty until the workspace changes. Re-fetch on false→true.
	// Seeded from the current value to avoid a redundant reload when already present.
	let prevHadLicense = !!$enterpriseLicense
	$effect(() => {
		const hasLicense = !!$enterpriseLicense
		if (hasLicense && !prevHadLicense && $workspaceStore) {
			loadTriggers($workspaceStore, workspaceLoadSeq)
		}
		prevHadLicense = hasLicense
	})

	let relevantTriggers = $derived.by(() => {
		const selectedScripts = new Set(
			selectedItems.filter((i) => i.kind === 'script').map((i) => i.path)
		)
		const selectedFlows = new Set(selectedItems.filter((i) => i.kind === 'flow').map((i) => i.path))
		return workspaceTriggers.filter((t) =>
			t.is_flow ? selectedFlows.has(t.script_path) : selectedScripts.has(t.script_path)
		)
	})

	let triggersByKind = $derived.by(() => {
		const out = new Map<TriggerKindLabel, WorkspaceTrigger[]>()
		for (const t of relevantTriggers) {
			const arr = out.get(t.kind) ?? []
			arr.push(t)
			out.set(t.kind, arr)
		}
		return Array.from(out.entries()).sort((a, b) => a[0].localeCompare(b[0]))
	})

	$effect(() => {
		for (const t of relevantTriggers) {
			if (t.kind !== 'schedule') continue
			const c = t.config as any
			const key = `${c.schedule}|${c.timezone}`
			if (schedulePreviews[key] || schedulePreviewsInFlight.has(key)) continue
			schedulePreviewsInFlight.add(key)
			ScheduleService.previewSchedule({
				requestBody: {
					schedule: c.schedule,
					timezone: c.timezone,
					cron_version: c.cron_version ?? 'v2'
				}
			})
				.then((dates) => {
					schedulePreviews = { ...schedulePreviews, [key]: dates.slice(0, 3) }
				})
				.catch(() => {})
				.finally(() => schedulePreviewsInFlight.delete(key))
		}
	})

	function triggerDetails(t: WorkspaceTrigger): Array<{ label: string; value: string }> {
		const c = t.config as any
		const out: Array<{ label: string; value: string }> = []
		const push = (label: string, v: any) => {
			if (v != null && v !== '') out.push({ label, value: String(v) })
		}
		switch (t.kind) {
			case 'http':
				push('Route', `${(c.http_method ?? '').toUpperCase()} /${c.route_path ?? ''}`)
				push('Auth', c.authentication_method)
				break
			case 'schedule':
				push('Cron', c.schedule)
				push('Timezone', c.timezone)
				break
			case 'websocket':
				push('URL', c.url)
				break
			case 'kafka':
				push('Resource', c.kafka_resource_path)
				push('Group', c.group_id)
				push('Topics', (Array.isArray(c.topics) ? c.topics : []).join(', '))
				break
			case 'nats':
				push('Resource', c.nats_resource_path)
				push('Subjects', (Array.isArray(c.subjects) ? c.subjects : []).join(', '))
				push('Jetstream', c.use_jetstream)
				break
			case 'sqs':
				push('Queue', c.queue_url)
				push('Resource', c.aws_resource_path)
				break
			case 'mqtt':
				push('Resource', c.mqtt_resource_path)
				push(
					'Topics',
					(Array.isArray(c.subscribe_topics) ? c.subscribe_topics : [])
						.map((x: any) => x?.topic ?? x)
						.join(', ')
				)
				break
			case 'gcp':
				push('Resource', c.gcp_resource_path)
				push('Topic', c.topic_id)
				push('Subscription', c.subscription_id)
				break
			case 'azure':
				push('Resource', c.azure_resource_path)
				push('Scope', c.scope_resource_id)
				push('Subscription', c.subscription_name)
				break
			case 'postgres':
				push('Resource', c.postgres_resource_path)
				push('Publication', c.publication_name)
				break
			case 'email':
				push('Email prefix', c.local_part ? `${c.local_part}@…` : undefined)
				break
		}
		return out
	}

	let runnableSummaryByPath = $derived.by(() => {
		const m = new Map<string, string | undefined>()
		for (const it of workspaceItems) {
			if (it.kind === 'script' || it.kind === 'flow') {
				m.set(`${it.kind}:${it.path}`, it.summary)
			}
		}
		return m
	})

	// Best-effort data table migrations for the bundle, editable in the drawer and
	// pushed on deploy. Regenerated when the bundle drawer opens.
	let migrationDrafts = $state<GeneratedMigration[]>([])
	let migrationsGenerating = $state(false)
	let migrationsSeq = 0

	async function regenerateMigrations(workspace: string) {
		const seq = ++migrationsSeq
		migrationsGenerating = true
		try {
			const seed: ItemRef[] = selectedItems
				.filter((i) => i.kind !== 'resource')
				.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path }))
			// Detection is independent of the final slug (data table refs aren't
			// relocated), so any placeholder slug works for this throwaway bundle.
			const bundle = await buildProjectBundle(
				seed,
				hubSlug || 'project',
				buildBundleDeps(workspace),
				[]
			)
			const usage = await detectDatatableTables(bundle.items)
			const drafts = await generateDatatableMigrations(workspace, usage)
			if (seq !== migrationsSeq) return
			migrationDrafts = drafts
		} catch (e: any) {
			if (seq === migrationsSeq) migrationDrafts = []
		} finally {
			if (seq === migrationsSeq) migrationsGenerating = false
		}
	}

	function openBundle() {
		hubName = hubName || folderProp
		bundleDrawer?.openDrawer()
		const workspace = $workspaceStore
		if (workspace) void regenerateMigrations(workspace)
	}
	function openTriggerUrl(kind: TriggerKindLabel): string | undefined {
		const ws = $workspaceStore
		if (!ws) return undefined
		return `${base}/${TRIGGER_KINDS[kind].route}?workspace=${ws}`
	}

	const ITEM_KIND_ROUTE: Record<ItemKind, string> = {
		script: 'scripts/get',
		flow: 'flows/get',
		app: 'apps/get',
		raw_app: 'apps_raw/get'
	}
	function openItemUrl(kind: ItemKind, path: string): string | undefined {
		const ws = $workspaceStore
		if (!ws || !path) return undefined
		return `${base}/${ITEM_KIND_ROUTE[kind]}/${path}?workspace=${ws}`
	}

	function sanitizeSlug(s: string): string {
		return s
			.toLowerCase()
			.replace(/[_\s]+/g, '-')
			.replace(/[^a-z0-9-]/g, '')
			.replace(/-+/g, '-')
			.replace(/^-+|-+$/g, '')
			.slice(0, 50)
			.replace(/-+$/g, '')
	}
	const SLUG_RE = /^[a-z0-9][a-z0-9-]{1,48}[a-z0-9]$/
	function isValidSlug(s: string): boolean {
		return SLUG_RE.test(s)
	}

	async function confirmBundle() {
		hubName = hubName.trim()
		hubSummary = hubSummary.trim()
		hubReadme = hubReadme.trim()
		const workspace = $workspaceStore
		if (!workspace) return
		// Capture the load sequence: a workspace switch during the draft request
		// bumps it (and resets selectedItems/relevantTriggers to the new workspace).
		const seq = workspaceLoadSeq
		try {
			const res = await fetch(`/api/w/${workspace}/hub/publish_draft${folderQs()}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({
					slug: hubSlug,
					name: hubName,
					summary: hubSummary || hubName,
					readme: hubReadme || undefined
				})
			})
			const text = await res.text()
			if (!res.ok) {
				sendUserToast(`Hub draft creation failed: ${text}`, true)
				return
			}
			// Abort if Hub didn't echo a slug — guessing here lands items under
			// a folder the Hub never locked.
			let returnedSlug: string | undefined
			try {
				const parsed = JSON.parse(text)
				if (typeof parsed?.slug === 'string') returnedSlug = parsed.slug
			} catch {}
			if (!returnedSlug) {
				sendUserToast(`Hub did not return a slug. Aborting publish to avoid path drift.`, true)
				return
			}
			// Workspace switched mid-request: publishing now would push the new
			// workspace's items/triggers into the old workspace's Hub draft. Abort.
			if (workspaceLoadSeq !== seq) {
				sendUserToast(`Workspace changed during publish — aborted to avoid mixing items.`, true)
				return
			}
			effectiveSlug = returnedSlug
		} catch (e: any) {
			sendUserToast(`Hub draft creation failed: ${e?.message ?? e}`, true)
			return
		}
		bundleDrawer?.closeDrawer()
		await deployAll(workspace)
	}

	let hubItemIds = $state<Record<string, number>>({})

	function buildBundleDeps(workspace: string): BundleDeps {
		return {
			fetchItem: async (ref: ItemRef): Promise<FetchedItem | undefined> => {
				try {
					if (ref.kind === 'script') {
						const s = await ScriptService.getScriptByPath({ workspace, path: ref.path })
						return {
							kind: 'script',
							path: ref.path,
							summary: s.summary,
							description: s.description ?? undefined,
							content: s.content,
							language: s.language,
							schema: s.schema,
							lock: s.lock ?? undefined,
							scriptKind: typeof s.kind === 'string' ? s.kind.toLowerCase() : 'script'
						}
					} else if (ref.kind === 'flow') {
						const f = await FlowService.getFlowByPath({ workspace, path: ref.path })
						return {
							kind: 'flow',
							path: ref.path,
							summary: f.summary,
							description: f.description ?? undefined,
							value: f.value,
							schema: f.schema
						}
					} else if (ref.kind === 'app') {
						const a = await AppService.getAppByPath({ workspace, path: ref.path })
						return { kind: 'app', path: ref.path, summary: a.summary, value: a.value }
					} else if (ref.kind === 'raw_app') {
						// Modern raw apps live in the `app` table: fetch source files +
						// runnables + the compiled bundle, and shape them into the `raw`
						// payload the Hub's RawAppView expects (JSON is valid YAML).
						const isModern = workspaceItems.some(
							(i) => i.kind === 'raw_app' && i.path === ref.path && i.appTable
						)
						if (isModern) {
							const a = await AppService.getAppByPath({ workspace, path: ref.path })
							const secret = await AppService.getPublicSecretOfLatestVersionOfApp({
								workspace,
								path: ref.path
							})
							// The compiled JS bundle is required; a missing one means the app
							// was never built/deployed, so fail loudly instead of pushing a blank app.
							const [jsRes, cssRes] = await Promise.all([
								fetch(`/api/w/${workspace}/apps/get_data/v/${secret}.js`, {
									credentials: 'include'
								}),
								fetch(`/api/w/${workspace}/apps/get_data/v/${secret}.css`, {
									credentials: 'include'
								})
							])
							if (!jsRes.ok) {
								throw new Error(`raw app ${ref.path} has no compiled bundle — deploy it first`)
							}
							const js = await jsRes.text()
							const css = cssRes.ok ? await cssRes.text() : ''
							const v: any = a.value ?? {}
							const content = JSON.stringify({
								files: { ...(v.files ?? {}), '/bundle.js': js, '/bundle.css': css },
								runnables: v.runnables ?? {}
							})
							return { kind: 'raw_app', path: ref.path, summary: a.summary, content }
						}
						const r = await fetch(`/api/w/${workspace}/raw_apps/get_data/0/${ref.path}`, {
							credentials: 'include'
						})
						if (!r.ok) return undefined
						return { kind: 'raw_app', path: ref.path, content: await r.text() }
					}
				} catch (e: any) {
					return undefined
				}
				return undefined
			},
			resolveResourceType: async (path: string): Promise<string | undefined> => {
				try {
					const r = await ResourceService.getResource({ workspace, path })
					return r.resource_type ?? undefined
				} catch (e: any) {
					return undefined
				}
			}
		}
	}

	async function pushBundledItem(workspace: string, slug: string, it: BundledItem): Promise<void> {
		const key = `${it.kind}:${it.path}`
		if (it.kind === 'script') {
			const resp = await postHub(workspace, '/hub/scripts', {
				summary: it.summary || it.newPath,
				app: slug,
				description: it.description ?? '',
				kind: it.scriptKind ?? 'script',
				content: it.content,
				language: it.language,
				schema: it.schema ?? undefined,
				lockfile: it.lock ?? undefined,
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') hubItemIds = { ...hubItemIds, [key]: resp.id }
		} else if (it.kind === 'flow') {
			const resp = await postHub(workspace, '/hub/flows', {
				flow: {
					summary: it.summary || it.newPath,
					description: it.description ?? undefined,
					value: it.value,
					schema: it.schema ?? undefined
				},
				apps: [],
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') hubItemIds = { ...hubItemIds, [key]: resp.id }
		} else if (it.kind === 'app') {
			await postHub(workspace, '/hub/apps', {
				app: it.value,
				apps: [],
				summary: it.summary || it.newPath,
				description: undefined,
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
		} else if (it.kind === 'raw_app') {
			const resp = await postHub(workspace, '/hub/raw_apps', {
				raw: it.content ?? '',
				apps: [],
				summary: it.summary || it.newPath,
				path: it.newPath,
				source_path: it.path,
				description: undefined,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') hubItemIds = { ...hubItemIds, [key]: resp.id }
		}
	}

	// Instance-side metadata that has no meaning on the Hub stub. Anything
	// `last_*` or `captured_*` is also dropped.
	const TRIGGER_CONFIG_BLACKLIST = new Set([
		'path',
		'script_path',
		'is_flow',
		'summary',
		'description',
		'workspace_id',
		'edited_by',
		'edited_at',
		'enabled',
		'extra_perms',
		'permissioned_as',
		'permissioned_as_email',
		'error',
		'error_handler_path',
		'error_handler_args',
		'test_runnable_args'
	])
	function stripTriggerConfig(config: Record<string, unknown>): Record<string, unknown> {
		const out: Record<string, unknown> = {}
		for (const [k, v] of Object.entries(config)) {
			if (TRIGGER_CONFIG_BLACKLIST.has(k)) continue
			if (k.startsWith('last_') || k.startsWith('captured_')) continue
			out[k] = v
		}
		return out
	}

	async function pushTriggers(
		workspace: string,
		slug: string,
		resourcePathMap: Map<string, string>,
		relevant: WorkspaceTrigger[]
	): Promise<void> {
		if (relevant.length === 0) return
		const pathMap = buildPathMap(
			relevant.map((t) => t.path),
			slug
		)
		const triggers: Array<Record<string, unknown>> = []
		const skipped: string[] = []
		for (const t of relevant) {
			const itemKind: ItemKind = t.is_flow ? 'flow' : 'script'
			const runnableKey = `${itemKind}:${t.script_path}`
			const hubId = hubItemIds[runnableKey]
			if (!hubId) {
				skipped.push(t.path)
				continue
			}
			const config = stripTriggerConfig(t.config)
			const field = TRIGGER_KINDS[t.kind]?.resourceField
			if (field && typeof config[field] === 'string') {
				config[field] = resourcePathMap.get(config[field] as string) ?? config[field]
			}
			triggers.push({
				path: pathMap.get(t.path) ?? t.path,
				kind: t.kind,
				summary: t.summary ?? null,
				description: (t.config as any)?.description ?? null,
				config,
				script_ask_id: t.is_flow ? null : hubId,
				flow_id: t.is_flow ? hubId : null
			})
		}
		if (skipped.length > 0) {
			sendUserToast(
				`Skipped ${skipped.length} trigger(s) whose runnable did not publish: ${skipped.join(', ')}`,
				true
			)
		}
		if (triggers.length === 0) return
		await postHub(workspace, '/hub/triggers', { triggers, project_slug: slug })
	}

	async function postHub(
		workspace: string,
		path: string,
		body: unknown
	): Promise<Record<string, any> | undefined> {
		const res = await fetch(`/api/w/${workspace}${path}${folderQs()}`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify(body)
		})
		const text = await res.text()
		if (!res.ok) throw new Error(text)
		try {
			return JSON.parse(text)
		} catch {
			return undefined
		}
	}

	const HIDDEN_RESOURCE_TYPES = new Set(['app_theme', 'state', 'cache'])

	function typesFromSchema(schema: any): string[] {
		const out = new Set<string>()
		const props = schema?.properties
		if (props && typeof props === 'object') {
			for (const key of Object.keys(props)) {
				const fmt = props[key]?.format
				if (typeof fmt === 'string' && fmt.startsWith('resource-')) {
					out.add(fmt.slice('resource-'.length))
				}
			}
		}
		return [...out]
	}

	let bundlePreview = $state<ProjectBundle | undefined>(undefined)
	let detectingResources = $state(false)
	// Data tables (→ tables) the current selection reads/writes, detected off the
	// same bundle preview. Drives the predeploy "Data table dependencies" summary;
	// the editable migration itself is generated in the bundle drawer.
	let datatableUsage = $state<Map<string, Set<string>>>(new Map())
	let detectingDatatables = $state(false)

	// `hasHardcoded` = pinned via $res: path (relocated as a stub); else input-only.
	type DependencyUsage =
		| { role: 'input'; label: string; kind: ItemKind; itemPath: string }
		| { role: 'hardcoded'; label: string; kind: ItemKind; path: string; itemPath: string }
		| { role: 'trigger'; label: string; triggerKind: TriggerKindLabel; path: string }
	interface DependencyType {
		resource_type: string
		hasHardcoded: boolean
		usages: DependencyUsage[]
	}
	let dependencyTypes = $derived.by(() => {
		const b = bundlePreview
		if (!b) return [] as DependencyType[]
		const stubByNewPath = new Map(b.resourceStubs.map((s) => [s.newPath, s]))
		const byType = new Map<string, DependencyType>()
		const ensure = (rt: string) => {
			let e = byType.get(rt)
			if (!e) {
				e = { resource_type: rt, hasHardcoded: false, usages: [] }
				byType.set(rt, e)
			}
			return e
		}
		for (const it of b.items) {
			const label = (it.summary?.trim() || it.path) ?? it.path
			const refs =
				it.kind === 'flow'
					? extractFlowRefs(it.value).filter((r) => r.kind === 'resource')
					: it.kind === 'app'
						? extractAppRefs(it.value)
						: extractScriptRefs(it.content ?? '')
			for (const r of refs) {
				const stub = stubByNewPath.get(r.path)
				if (!stub || HIDDEN_RESOURCE_TYPES.has(stub.resource_type)) continue
				const e = ensure(stub.resource_type)
				e.hasHardcoded = true
				e.usages.push({
					role: 'hardcoded',
					label,
					kind: it.kind,
					path: stub.originalPath,
					itemPath: it.path
				})
			}
			for (const t of typesFromSchema(it.schema)) {
				if (HIDDEN_RESOURCE_TYPES.has(t)) continue
				ensure(t).usages.push({ role: 'input', label, kind: it.kind, itemPath: it.path })
			}
		}
		// Resources referenced only by a trigger (no item uses them in code).
		const stubByOriginal = new Map(b.resourceStubs.map((s) => [s.originalPath, s]))
		for (const t of relevantTriggers) {
			const rp = triggerResourcePath(t)
			const stub = rp ? stubByOriginal.get(rp) : undefined
			if (!stub || HIDDEN_RESOURCE_TYPES.has(stub.resource_type)) continue
			ensure(stub.resource_type).usages.push({
				role: 'trigger',
				label: t.summary?.trim() || t.path,
				triggerKind: t.kind,
				path: stub.originalPath
			})
		}
		return [...byType.values()].sort((a, b) => a.resource_type.localeCompare(b.resource_type))
	})

	// Preview-only cache: toggling checkboxes re-runs the closure walk, but item
	// contents don't change mid-session. deployAll bypasses this and fetches fresh.
	const previewItemCache = new Map<string, Promise<FetchedItem | undefined>>()
	const previewTypeCache = new Map<string, Promise<string | undefined>>()
	function cachedBundleDeps(workspace: string): BundleDeps {
		const deps = buildBundleDeps(workspace)
		return {
			fetchItem: (ref) => {
				const key = `${ref.kind}:${ref.path}`
				let p = previewItemCache.get(key)
				if (!p) {
					p = deps.fetchItem(ref)
					previewItemCache.set(key, p)
				}
				return p
			},
			resolveResourceType: (path) => {
				let p = previewTypeCache.get(path)
				if (!p) {
					p = deps.resolveResourceType(path)
					previewTypeCache.set(path, p)
				}
				return p
			}
		}
	}

	$effect(() => {
		const workspace = $workspaceStore
		selectedItemKeys
		if (!workspace || phase !== 'predeploy') {
			bundlePreview = undefined
			datatableUsage = new Map()
			return
		}
		let cancelled = false
		detectingResources = true
		detectingDatatables = true
		const slug = hubSlug
		const seed: ItemRef[] = selectedItems
			.filter((i) => i.kind !== 'resource')
			.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path }))
		const triggerResources = relevantTriggers
			.map(triggerResourcePath)
			.filter((p): p is string => !!p)
		// Debounce so rapid checkbox toggles coalesce into one walk.
		const timer = setTimeout(() => {
			buildProjectBundle(seed, slug, cachedBundleDeps(workspace), triggerResources)
				.then((b) => {
					if (cancelled) return
					bundlePreview = b
					// Detect data table usage off the same fetched items.
					detectDatatableTables(b.items)
						.then((usage) => {
							if (!cancelled) datatableUsage = usage
						})
						.finally(() => {
							if (!cancelled) detectingDatatables = false
						})
				})
				.finally(() => {
					if (!cancelled) detectingResources = false
				})
		}, 250)
		return () => {
			cancelled = true
			clearTimeout(timer)
		}
	})

	// Builtin types (git_repository, ...) aren't in resource_type — push with empty schema.
	async function pushResourceTypes(
		workspace: string,
		slug: string,
		types: string[]
	): Promise<number> {
		const results = await Promise.all(
			types.map(async (name) => {
				let schema: unknown = undefined
				let description: string | undefined = undefined
				try {
					const rt = await ResourceService.getResourceType({ workspace, path: name })
					schema = rt.schema ?? undefined
					description = rt.description ?? undefined
				} catch (e: any) {}
				try {
					await postHub(workspace, '/hub/resource_types', {
						name,
						schema,
						description,
						project_slug: slug
					})
					return 0
				} catch (e: any) {
					sendUserToast(`Resource type ${name} push failed: ${e?.message ?? e}`, true)
					return 1
				}
			})
		)
		return results.reduce((a: number, b) => a + b, 0)
	}

	// `workspace` is captured by confirmBundle before the draft is created, so the
	// deploy stays bound to the draft's source_id — a mid-deploy switch can't split.
	async function deployAll(workspace: string) {
		const slug = hubSlug
		// Snapshot the selection up-front so a workspace switch mid-deploy can't
		// write stale state into the new workspace.
		const itemsSnapshot = selectedItems.slice()
		const triggersSnapshot = relevantTriggers.slice()
		hubItemIds = {}
		deploying = true
		let failures = 0
		try {
			const seed: ItemRef[] = itemsSnapshot
				.filter((i) => i.kind !== 'resource')
				.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path }))
			const triggerResources = triggersSnapshot
				.map(triggerResourcePath)
				.filter((p): p is string => !!p)
			const bundle = await buildProjectBundle(
				seed,
				slug,
				buildBundleDeps(workspace),
				triggerResources
			)
			// Full path map (incl. unresolved) so a trigger's resource path is always
			// relocated — never leaks the publisher's original private path to the Hub.
			const resourcePathMap = bundle.pathMap

			if (bundle.unresolved.length > 0) {
				sendUserToast(
					`Skipped ${bundle.unresolved.length} unresolved reference(s): ${bundle.unresolved.join(', ')}`,
					true
				)
			}

			// Types come from $res: stubs AND schema inputs (resource-<type>).
			const inputTypes = bundle.items
				.flatMap((i) => typesFromSchema(i.schema))
				.filter((t) => !HIDDEN_RESOURCE_TYPES.has(t))
			const types = [
				...new Set([...bundle.resourceStubs.map((s) => s.resource_type), ...inputTypes])
			]
			const depFailures = await pushResourceTypes(workspace, slug, types)

			// Input-type deps with no path get a conventional f/<slug>/<type> stub.
			const stubsByPath = new Map<string, { path: string; resource_type: string }>()
			for (const s of bundle.resourceStubs)
				stubsByPath.set(s.newPath, { path: s.newPath, resource_type: s.resource_type })
			for (const t of inputTypes) {
				const path = `f/${slug}/${t}`
				if (!stubsByPath.has(path)) stubsByPath.set(path, { path, resource_type: t })
			}
			const stubs = [...stubsByPath.values()]
			if (stubs.length > 0) {
				try {
					await postHub(workspace, '/hub/resources', { resources: stubs, project_slug: slug })
				} catch (e: any) {
					sendUserToast(`Resource sync failed: ${e?.message ?? e}`, true)
					failures++
				}
			}
			failures += depFailures
			if (failures > 0) {
				sendUserToast(
					`Resource dependency sync failed — items not published to avoid broken references.`,
					true
				)
				return
			}

			for (const it of bundle.items) {
				// Stop writing item status / Hub IDs into a workspace the user has
				// switched away from mid-publish.
				if ($workspaceStore !== workspace) return
				const key = `${it.kind}:${it.path}`
				deploymentStatus = { ...deploymentStatus, [key]: { status: 'loading' } }
				try {
					await pushBundledItem(workspace, slug, it)
					deploymentStatus = { ...deploymentStatus, [key]: { status: 'deployed' } }
				} catch (e: any) {
					failures++
					deploymentStatus = {
						...deploymentStatus,
						[key]: { status: 'failed', error: e?.message ?? String(e) }
					}
				}
			}
			// A re-bundle clears the Hub-side embed (idempotent replace), so re-push it
			// for any raw app that is already public — keeps the live iframe in sync
			// without forcing an unpublish/share round-trip. Updates by hub id, safe in parallel.
			await Promise.all(
				bundle.items
					.filter((it) => it.kind === 'raw_app')
					.map(async (it) => {
						const hubId = hubItemIds[`${it.kind}:${it.path}`]
						const src = itemsSnapshot.find((i) => i.kind === 'raw_app' && i.path === it.path)
						if (hubId && src?.published && src?.publicUrl) {
							try {
								await pushRawAppEmbed(workspace, hubId, src.publicUrl)
							} catch (e: any) {
								sendUserToast(`Failed to sync iframe for ${it.path}: ${e?.message ?? e}`, true)
							}
						}
					})
			)
			try {
				await pushTriggers(workspace, slug, resourcePathMap, triggersSnapshot)
			} catch (e: any) {
				sendUserToast(`Trigger sync failed: ${e?.message ?? e}`, true)
				failures++
			}

			// Full-set sync: always push (an empty list clears the Hub's migrations on
			// a re-deploy). The Hub drops empty-SQL entries, so disabled placeholders
			// don't persist.
			try {
				await postHub(workspace, '/hub/migrations', {
					migrations: migrationDrafts.map((m) => ({
						datatable_name: m.datatable_name,
						sql: m.sql,
						enabled: m.enabled
					})),
					project_slug: slug
				})
			} catch (e: any) {
				sendUserToast(`Data table migration sync failed: ${e?.message ?? e}`, true)
				failures++
			}

			await sleep(150)
			if ($workspaceStore !== workspace) return
			deploymentStatus = {}
			recordings = {}
			// Deterministic baseline so a transient Hub read failure can't leave the
			// UI stuck in `predeploy`; rehydrate then upgrades to authoritative state.
			draftItems = itemsSnapshot.map((i) => ({ ...i, rec: 'none' }))
			phase = 'draft'
			await rehydrateFromHub(workspace, folderProp, ++workspaceLoadSeq)
			if (failures > 0) {
				sendUserToast(`Draft pushed with ${failures} failed item(s).`, true)
			} else {
				sendUserToast(`Draft created on the Hub. Add recordings before submitting for review.`)
			}
		} finally {
			deploying = false
		}
	}

	let submitting = $state(false)
	async function submitForReview() {
		const workspace = $workspaceStore
		const slug = hubSlug
		if (!workspace || !slug) return
		submitting = true
		try {
			const res = await fetch(
				`/api/w/${workspace}/hub/projects/${encodeURIComponent(slug)}/submit${folderQs()}`,
				{
					method: 'POST',
					credentials: 'include',
					headers: { 'Content-Type': 'application/json' },
					body: '{}'
				}
			)
			if (!res.ok) {
				sendUserToast(`Submit for review failed: ${await res.text()}`, true)
				return
			}
			phase = 'under_review'
			sendUserToast('Submitted for review by the Windmill team.')
		} finally {
			submitting = false
		}
	}
	let syncing = $state(false)
	async function syncWithHub() {
		const workspace = $workspaceStore
		if (!workspace) return
		syncing = true
		try {
			if (phase === 'draft') {
				await loadWorkspace(workspace, workspaceLoadSeq)
				const prev = new Map(draftItems.map((i) => [i.key, { rec: i.rec }]))
				draftItems = workspaceItems
					.filter((i) => prev.has(i.key))
					.map((i) => ({ ...i, rec: prev.get(i.key)?.rec ?? 'none' }))
			} else {
				// under_review / live: re-fetch the Hub project to pick up an
				// admin status change (under_review -> live).
				const before = phase
				await rehydrateFromHub(workspace, folderProp, workspaceLoadSeq)
				sendUserToast(
					phase === before
						? 'Still waiting for review.'
						: phase === 'live'
							? 'Approved — your project is now live.'
							: `Status updated: ${phase}.`
				)
			}
		} catch (e: any) {
			sendUserToast(`Sync failed: ${e?.message ?? e}`, true)
		} finally {
			syncing = false
		}
	}
	function startNewDraft() {
		draftItems = []
		recordings = {}
		phase = 'predeploy'
	}

	async function openRecord(it: DeployItem) {
		const seq = ++recordRunSeq
		recordTarget = it
		recordArgs = {}
		recordValid = true
		recordSchema = emptySchema()
		recordSchemaLoading = true
		runState = 'idle'
		runJobId = undefined
		runResult = undefined
		runError = undefined
		recordDrawer?.openDrawer()
		const workspace = $workspaceStore
		if (!workspace) {
			recordSchemaLoading = false
			return
		}
		try {
			if (it.kind === 'script') {
				const s = await ScriptService.getScriptByPath({ workspace, path: it.path })
				if (seq !== recordRunSeq) return
				recordSchema = (s.schema as Record<string, any>) ?? emptySchema()
			} else if (it.kind === 'flow') {
				const f = await FlowService.getFlowByPath({ workspace, path: it.path })
				if (seq !== recordRunSeq) return
				recordSchema = (f.schema as Record<string, any>) ?? emptySchema()
			}
		} catch (e: any) {
			if (seq !== recordRunSeq) return
			sendUserToast(`Failed to load schema: ${e?.message ?? e}`, true)
		} finally {
			if (seq === recordRunSeq) recordSchemaLoading = false
		}
	}
	async function runJob() {
		const it = recordTarget
		const workspace = $workspaceStore
		if (!it || !workspace) return
		const seq = ++recordRunSeq
		runState = 'running'
		runJobId = undefined
		runResult = undefined
		runError = undefined
		try {
			let jobId: string
			if (it.kind === 'script') {
				jobId = await JobService.runScriptByPath({
					workspace,
					path: it.path,
					requestBody: recordArgs
				})
			} else if (it.kind === 'flow') {
				jobId = await JobService.runFlowByPath({
					workspace,
					path: it.path,
					requestBody: recordArgs
				})
			} else {
				if (seq === recordRunSeq) runState = 'idle'
				return
			}
			if (seq !== recordRunSeq) return
			runJobId = jobId
			await pollJobUntilComplete(workspace, jobId, seq)
		} catch (e: any) {
			if (seq !== recordRunSeq) return
			runState = 'failed'
			runError = `Failed to start: ${e?.message ?? e}`
		}
	}
	async function pollJobUntilComplete(workspace: string, jobId: string, seq: number) {
		// First check immediately (fast scripts complete in ms), then back off to 2s.
		const deadline = Date.now() + 5 * 60_000
		let interval = 250
		while (Date.now() < deadline) {
			if (seq !== recordRunSeq) return
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace,
					id: jobId
				})
				if (seq !== recordRunSeq) return
				if (r.completed) {
					runResult = r.result
					if (r.success) {
						runState = 'success'
					} else {
						runState = 'failed'
						runError = typeof r.result === 'string' ? r.result : JSON.stringify(r.result)
					}
					return
				}
			} catch (e: any) {
				if (seq !== recordRunSeq) return
				runState = 'failed'
				runError = `Polling failed: ${e?.message ?? e}`
				return
			}
			await sleep(interval)
			interval = Math.min(interval * 2, 2000)
		}
		if (seq !== recordRunSeq) return
		runState = 'failed'
		runError = 'Timed out after 5 minutes'
	}
	async function buildScriptRecording(workspace: string, it: DeployItem, jobId: string) {
		const s = await ScriptService.getScriptByPath({ workspace, path: it.path })
		const job = await JobService.getCompletedJob({ workspace, id: jobId })
		const initial_job = { ...(job as any), type: 'CompletedJob' }
		const events = [{ t: 0, data: { completed: true, job: initial_job } }]
		const duration = (initial_job.duration_ms as number) ?? 0
		return {
			version: 1,
			type: 'script' as const,
			recorded_at: new Date().toISOString(),
			script_path: it.path,
			total_duration_ms: duration,
			code: s.content,
			language: s.language,
			args: (job.args ?? {}) as Record<string, any>,
			schema: s.schema,
			job: { initial_job, events }
		}
	}

	async function buildFlowRecording(workspace: string, it: DeployItem, jobId: string) {
		const f = await FlowService.getFlowByPath({ workspace, path: it.path })
		const root = (await JobService.getCompletedJob({ workspace, id: jobId })) as any
		const jobs: Record<string, { initial_job: any; events: any[] }> = {}
		const collect = async (j: any) => {
			const stamped = { ...j, type: 'CompletedJob' }
			jobs[j.id] = {
				initial_job: stamped,
				events: [{ t: 0, data: { completed: true, job: stamped } }]
			}
			const modules = (j.flow_status?.modules ?? []).filter(
				(m: any) => m.job && typeof m.job === 'string'
			)
			// Sub-jobs at the same level are independent reads.
			await Promise.all(
				modules.map(async (m: any) => {
					try {
						const sub = (await JobService.getCompletedJob({ workspace, id: m.job })) as any
						await collect(sub)
					} catch {
						/* sub-job missing — skip */
					}
				})
			)
		}
		await collect(root)
		return {
			version: 1,
			recorded_at: new Date().toISOString(),
			flow_path: it.path,
			total_duration_ms: (root.duration_ms as number) ?? 0,
			flow: {
				path: it.path,
				value: f.value,
				schema: f.schema ?? { type: 'object', properties: {}, required: [] },
				summary: f.summary ?? '',
				archived: false,
				edited_at: '',
				edited_by: '',
				extra_perms: {}
			},
			jobs
		}
	}

	async function saveRecording() {
		const it = recordTarget
		const workspace = $workspaceStore
		if (!it || !runJobId || runState !== 'success' || !workspace) return
		const hubId = hubItemIds[it.key]
		if (!hubId) {
			sendUserToast(`Push the bundle to the Hub first before saving recordings`, true)
			return
		}
		if (it.kind !== 'script' && it.kind !== 'flow') {
			sendUserToast(`Recordings only supported for script/flow`, true)
			return
		}
		try {
			const recording =
				it.kind === 'script'
					? await buildScriptRecording(workspace, it, runJobId)
					: await buildFlowRecording(workspace, it, runJobId)
			const path = it.kind === 'script' ? 'scripts' : 'flows'
			await postHub(workspace, `/hub/${path}/${hubId}/recording`, {
				recording,
				project_slug: hubSlug
			})
			recordings = { ...recordings, [it.key]: runJobId }
			patchItem(it.key, { rec: 'recorded' })
			sendUserToast(`Recording saved — job ${runJobId}`)
			recordDrawer?.closeDrawer()
		} catch (e: any) {
			sendUserToast(`Failed to save recording: ${e?.message ?? e}`, true)
		}
	}

	function openPublish(it: DeployItem) {
		publishTarget = it
		publishDrawer?.openDrawer()
	}
	// Set the Hub raw app's live-iframe URL (or clear it with null). The Hub renders
	// from external_embed_url; project_slug scopes ownership.
	async function pushRawAppEmbed(workspace: string, hubId: number, url: string | null) {
		await postHub(workspace, `/hub/raw_apps/${hubId}/embed`, {
			external_embed_url: url,
			project_slug: hubSlug
		})
	}

	// Flip an app/raw app between public (anonymous) and private (publisher) and keep
	// the Hub raw-app iframe in sync. Returns the resolved public URL when shared.
	async function setAppShared(
		workspace: string,
		it: DeployItem,
		shared: boolean
	): Promise<string | null> {
		const app = await AppService.getAppByPath({ workspace, path: it.path })
		const policy = {
			...(app.policy ?? {}),
			execution_mode: (shared ? 'anonymous' : 'publisher') as 'anonymous' | 'publisher'
		}
		await AppService.updateApp({
			workspace,
			path: it.path,
			requestBody: { policy, deployment_message: shared ? 'Share as iframe' : 'Unshare iframe' }
		})
		const url = shared ? ((await resolvePublicUrl(workspace, it.path)) ?? null) : null
		if (it.kind === 'raw_app') {
			const hubId = hubItemIds[it.key]
			if (!hubId) {
				if (shared) sendUserToast('Push the bundle to the Hub first to share the live iframe', true)
			} else if (!shared) {
				await pushRawAppEmbed(workspace, hubId, null)
			} else if (url) {
				await pushRawAppEmbed(workspace, hubId, url)
			}
		}
		return url
	}

	async function confirmPublish() {
		const it = publishTarget
		const workspace = $workspaceStore
		if (!it || !workspace || !canPublishApp(it.kind)) return
		publishing = true
		try {
			const url = await setAppShared(workspace, it, true)
			patchItem(it.key, { published: true, publicUrl: url ?? undefined })
			sendUserToast(`${it.path} is now public`)
			publishDrawer?.closeDrawer()
		} catch (e: any) {
			sendUserToast(`Failed to publish: ${e?.message ?? e}`, true)
		} finally {
			publishing = false
		}
	}
	async function unpublishApp(it: DeployItem) {
		const workspace = $workspaceStore
		if (!workspace || !canPublishApp(it.kind)) return
		try {
			await setAppShared(workspace, it, false)
			patchItem(it.key, { published: false, publicUrl: undefined })
			sendUserToast('App unpublished')
		} catch (e: any) {
			sendUserToast(`Failed to unpublish: ${e?.message ?? e}`, true)
		}
	}
	async function copyIframeSnippet(url: string) {
		const snippet = `<iframe src="${url}" width="100%" height="600" frameborder="0"></iframe>`
		try {
			await navigator.clipboard.writeText(snippet)
			sendUserToast('Iframe snippet copied to clipboard')
		} catch {
			sendUserToast('Failed to copy snippet', true)
		}
	}
</script>

<div>
	<WorkspaceDeployLayout
		{items}
		selectedItems={selectedItemKeys}
		{deploymentStatus}
		hideSelection={phase !== 'predeploy'}
		{allSelected}
		onToggleItem={toggleItem}
		onSelectAll={selectAll}
		onDeselectAll={deselectAll}
		emptyMessage={loading ? 'Loading project items…' : 'No items to publish'}
	>
		{#snippet header()}
			{@const stepNum =
				phase === 'predeploy' ? 1 : phase === 'draft' ? 2 : phase === 'under_review' ? 3 : 4}
			<div class="flex flex-col gap-2 w-full pb-4">
				<ol
					class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 text-xs text-secondary"
				>
					<span class="text-sm font-semibold text-primary">
						How to publish your project to the Hub
					</span>
					<li class={stepNum === 1 ? 'text-primary' : stepNum > 1 ? 'opacity-60' : ''}>
						<span class="font-mono text-emphasis">{stepNum > 1 ? '✓' : '1.'}</span>
						<span class="font-semibold text-primary">Bundle your project</span> — creates a draft on
						the Hub with every selected script, flow, app and resource from this folder.
					</li>
					<li class={stepNum === 2 ? 'text-primary' : stepNum > 2 ? 'opacity-60' : 'opacity-40'}>
						<span class="font-mono text-emphasis">{stepNum > 2 ? '✓' : '2.'}</span>
						<span class="font-semibold text-primary">Generate iframes &amp; recordings</span> — share
						public apps as iframes and capture one execution per script/flow.
					</li>
					<li class={stepNum === 3 ? 'text-primary' : stepNum > 3 ? 'opacity-60' : 'opacity-40'}>
						<span class="font-mono text-emphasis">{stepNum > 3 ? '✓' : '3.'}</span>
						<span class="font-semibold text-primary">Submit for review</span> — send the bundle for approval.
					</li>
				</ol>
				<div class="flex flex-wrap items-center gap-2 pt-4">
					{#if phase === 'predeploy'}
						<span class="text-sm font-semibold text-primary"> Step 1: Bundle your project </span>
					{:else if phase === 'draft'}
						<span class="text-sm font-semibold text-primary">
							Step 2: Generate iframes &amp; recordings
						</span>
					{:else if phase === 'under_review'}
						<span class="text-sm font-semibold text-primary">Step 3: Awaiting review</span>
					{:else}
						<span class="text-sm font-semibold text-primary">Live on the Hub</span>
					{/if}
					{#if phase !== 'predeploy'}
						<Badge color="transparent" class="font-semibold">
							<Cloud size={14} class="mr-1" />
							<span class="text-secondary">on Hub:</span>
							<span class="text-emphasis">{hubName || hubSlug}</span>
						</Badge>
						<a
							href={hubUrl}
							target="_blank"
							rel="noopener noreferrer"
							class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
						>
							<ExternalLink size={12} /> Open in Hub
						</a>
					{/if}
					<div class="ml-auto flex items-center gap-2">
						{#if phase === 'predeploy'}
							<Button
								variant="accent"
								loading={deploying}
								disabled={selectedItems.length === 0}
								startIcon={{ icon: Cloud }}
								onclick={openBundle}
							>
								Create Hub draft ({selectedItems.length})
							</Button>
						{:else if phase === 'draft'}
							<Button
								variant="accent"
								loading={submitting}
								startIcon={{ icon: Check }}
								onclick={submitForReview}
							>
								Submit for review
							</Button>
						{:else if phase === 'under_review'}
							<Button
								size="xs"
								variant="subtle"
								loading={syncing}
								startIcon={{ icon: RotateCcw }}
								iconOnly
								title="Refresh review status"
								onclick={syncWithHub}
							/>
						{:else}
							<Button variant="accent" startIcon={{ icon: RotateCcw }} onclick={startNewDraft}>
								New draft
							</Button>
						{/if}
					</div>
				</div>
				{#if phase === 'predeploy'}
					<div class="flex flex-col gap-1 pb-3">
						<span class="text-xs text-secondary">
							Bundling creates a draft project on the Hub from the selected scripts, flows and apps
							of <span class="font-mono">{selectedFolder}/</span>.
							{selectedItems.length} of {filteredWorkspaceItems.length} items selected.
						</span>
					</div>
					<div class="flex flex-wrap items-center gap-2 text-xs">
						<span class="font-semibold text-primary shrink-0">
							Resource dependencies
							{#if detectingResources}
								<Loader2 size={11} class="inline animate-spin text-hint" />
							{:else}
								<span class="text-hint font-normal">({dependencyTypes.length})</span>
							{/if}
							<Tooltip>
								Resource types the selected items depend on (whether passed as inputs or referenced
								by a hardcoded path). Synced to the Hub so a fork knows what credentials it needs to
								fill.
							</Tooltip>
						</span>
						{#if dependencyTypes.length === 0}
							<span class="text-[11px] text-hint">
								No resource references detected in the current selection.
							</span>
						{:else}
							{#each dependencyTypes as r (r.resource_type)}
								<span
									class="inline-flex items-center gap-1 rounded border px-1.5 py-0.5 font-mono text-[11px] text-secondary {r.hasHardcoded
										? 'border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40'
										: 'bg-surface'}"
								>
									{r.resource_type}
								</span>
							{/each}
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="ml-auto"
								onclick={() => resourceDrawer?.openDrawer()}
							>
								View details
							</Button>
						{/if}
					</div>
					<div class="flex flex-wrap items-center gap-2 text-xs">
						<span class="font-semibold text-primary shrink-0">
							Data table dependencies
							{#if detectingDatatables}
								<Loader2 size={11} class="inline animate-spin text-hint" />
							{:else}
								<span class="text-hint font-normal">({datatableUsage.size})</span>
							{/if}
							<Tooltip>
								Data tables the selected items read or write. A best-effort CREATE TABLE migration
								for these is generated in the bundle step and shipped with the project, so a fork
								can recreate the tables it needs.
							</Tooltip>
						</span>
						{#if datatableUsage.size === 0}
							<span class="text-[11px] text-hint">
								No data table usage detected in the current selection.
							</span>
						{:else}
							{#each [...datatableUsage] as [dt, tables] (dt)}
								<span
									class="inline-flex items-center gap-1 rounded border bg-surface px-1.5 py-0.5 font-mono text-[11px] text-secondary"
								>
									{dt}
									{#if tables.size > 0}
										<span class="text-hint">×{tables.size}</span>
									{/if}
								</span>
							{/each}
						{/if}
					</div>
				{/if}
				{#if phase === 'draft'}
					<div class="flex flex-col gap-1 pb-3">
						<span class="text-xs text-secondary">
							A recording captures one real run of a script or flow — inputs, logs, step outputs and
							result — replayable on the Hub so visitors see it work before forking. Public apps can
							also be shared as live iframes. Optional, but recommended.
						</span>
					</div>
				{/if}
				{#if phase === 'predeploy'}
					<div class="flex flex-wrap items-center gap-2 text-xs">
						<span class="font-semibold text-primary shrink-0">
							Triggers
							{#if triggersLoading}
								<Loader2 size={11} class="inline animate-spin text-hint" />
							{:else}
								<span class="text-hint font-normal">({relevantTriggers.length})</span>
							{/if}
						</span>
						{#if relevantTriggers.length === 0}
							<span class="text-[11px] text-hint">No triggers reference the selected items.</span>
						{:else}
							{#each triggersByKind as [kind, triggers] (kind)}
								<span
									class="inline-flex items-center gap-1 rounded border bg-surface px-1.5 py-0.5 font-mono text-[11px] text-secondary"
								>
									{TRIGGER_KINDS[kind].badge}
									<span class="text-hint">×{triggers.length}</span>
								</span>
							{/each}
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="ml-auto"
								onclick={() => triggerDrawer?.openDrawer()}
							>
								View details
							</Button>
						{/if}
					</div>
				{/if}
				{#if phase === 'under_review'}
					<div
						class="flex items-start gap-2 rounded-md border border-blue-300 bg-blue-50 p-3 text-xs text-blue-900 dark:border-blue-800 dark:bg-blue-950/40 dark:text-blue-100"
					>
						<TriangleAlert size={14} class="mt-0.5 shrink-0 text-blue-600 dark:text-blue-400" />
						<div class="flex flex-col gap-1">
							<span class="font-semibold">Locked while under review</span>
							<span>
								The Windmill team is reviewing this submission. Editing, recording, and sharing
								actions are disabled. Estimated turnaround: 1-2 business days.
							</span>
						</div>
					</div>
				{/if}
				{#if phase === 'draft'}
					{@const recordedCount = recordableItems.filter((i) => i.rec === 'recorded').length}
					{@const pct = recordableItems.length
						? Math.round((recordedCount / recordableItems.length) * 100)
						: 0}
					<div class="flex items-center gap-2 self-end text-[11px] text-tertiary">
						<span class="font-mono">{recordedCount}/{recordableItems.length}</span>
						<div class="h-1 w-24 overflow-hidden rounded bg-surface-tertiary">
							<div
								class="h-full {allRecorded ? 'bg-green-500' : 'bg-hint'} transition-all"
								style="width: {pct}%"
							></div>
						</div>
						<span class={allRecorded ? 'text-green-700 dark:text-green-400' : 'text-hint'}>
							{allRecorded ? 'Full recordings' : 'Recordings recommended'}
						</span>
					</div>
				{/if}
			</div>
		{/snippet}

		{#snippet itemSummary(item)}
			{@const it = item as DeployItem}
			<span class="truncate">
				{it.summary?.trim() || it.path}
			</span>
		{/snippet}

		{#snippet itemActions(item)}
			{@const it = item as DeployItem}
			{#if phase !== 'predeploy' && canRecord(it.kind)}
				{#if it.rec === 'recorded'}
					<Badge color="green" size="xs">
						<Check size={10} class="mr-0.5" />Recorded
					</Badge>
					{#if recordings[it.key]}
						<a
							href={`/run/${recordings[it.key]}?workspace=${$workspaceStore}`}
							target="_blank"
							rel="noopener noreferrer"
							class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
						>
							<ExternalLink size={12} /> See recording
						</a>
					{/if}
					{#if phase === 'draft'}
						<Button
							size="xs"
							variant="subtle"
							startIcon={{ icon: RotateCcw }}
							onclick={() => openRecord(it)}
						>
							Re-record
						</Button>
					{/if}
				{:else if phase === 'draft'}
					<Badge color="yellow" size="xs">No recording</Badge>
					<Button
						size="xs"
						variant="subtle"
						startIcon={{ icon: Play }}
						onclick={() => openRecord(it)}
					>
						Add recording
					</Button>
				{:else}
					<Badge color="yellow" size="xs">No recording</Badge>
				{/if}
			{/if}
			{#if phase !== 'predeploy' && canPublishApp(it.kind)}
				{#if it.published && it.publicUrl}
					<Badge color="green" size="xs">
						<Globe size={10} class="mr-0.5" />Public
					</Badge>
					<a
						href={it.publicUrl}
						target="_blank"
						rel="noopener noreferrer"
						class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
					>
						<ExternalLink size={12} /> Open
					</a>
					<Button
						size="xs"
						variant="subtle"
						startIcon={{ icon: Copy }}
						onclick={() => copyIframeSnippet(it.publicUrl!)}
					>
						Copy iframe
					</Button>
					{#if phase !== 'under_review'}
						<Button size="xs" variant="subtle" onclick={() => unpublishApp(it)}>Unpublish</Button>
					{/if}
				{:else if phase !== 'under_review'}
					<Button
						size="xs"
						variant="subtle"
						startIcon={{ icon: Globe }}
						onclick={() => openPublish(it)}
					>
						Share as iframe
					</Button>
				{/if}
			{/if}
		{/snippet}

		{#snippet footer()}
			<div class="flex items-center justify-end gap-3">
				{#if phase === 'predeploy'}
					<span class="text-[11px] text-hint">
						Select the items to include — all selected by default.
					</span>
				{:else if phase === 'draft'}
					<span class="text-[11px] text-hint">
						{#if allRecorded}
							All scripts and flows have a recording — best chance of approval and featuring.
						{:else}
							{recordableItems.filter((i) => i.rec === 'recorded').length} of {recordableItems.length}
							recorded. Bundles with full recordings get approved faster and featured on the public Hub.
						{/if}
					</span>
				{:else if phase === 'under_review'}
					<span class="text-[11px] text-hint">
						Waiting for the Windmill team to review the submission.
					</span>
				{:else}
					<span class="text-[11px] text-hint"> Iterate further by starting a new draft. </span>
				{/if}
			</div>
		{/snippet}
	</WorkspaceDeployLayout>
</div>

<Drawer bind:this={recordDrawer} size="600px" on:close={() => recordRunSeq++}>
	<DrawerContent
		title={recordTarget ? `Record — ${recordTarget.path}` : 'Record'}
		on:close={() => recordDrawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-3">
			<p class="text-xs text-secondary">
				Run this {recordTarget?.kind} once with the inputs below. The full execution — args, logs, intermediate
				step outputs and final result — is saved as a <b>replayable recording</b> shown on the Hub
				page. Visitors can step through it to see how the {recordTarget?.kind} works without running
				anything themselves.
			</p>

			{#if runState !== 'idle'}
				<div
					class="sticky top-0 z-10 flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 shadow-sm"
				>
					<div class="flex items-center gap-2 text-sm">
						{#if runState === 'running'}
							<Loader2 size={14} class="animate-spin text-blue-600 dark:text-blue-400" />
							<span class="font-semibold">Running…</span>
						{:else if runState === 'success'}
							<Check size={14} class="text-green-600 dark:text-green-400" />
							<span class="font-semibold text-green-700 dark:text-green-300">
								Execution succeeded
							</span>
						{:else}
							<X size={14} class="text-red-600 dark:text-red-400" />
							<span class="font-semibold text-red-700 dark:text-red-300">Execution failed</span>
						{/if}
						{#if runJobId}
							<a
								href={`/run/${runJobId}?workspace=${$workspaceStore}`}
								target="_blank"
								rel="noopener noreferrer"
								class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
							>
								<ExternalLink size={12} /> Open job
							</a>
						{/if}
					</div>
					{#if runState === 'success' && runResult !== undefined}
						<div class="flex flex-col gap-1 text-xs">
							<span class="text-secondary">Result preview:</span>
							<pre class="max-h-40 overflow-auto rounded bg-surface p-2 font-mono text-[11px]"
								>{JSON.stringify(runResult, null, 2)}</pre
							>
						</div>
					{:else if runState === 'failed' && runError}
						<pre
							class="max-h-40 overflow-auto rounded bg-surface p-2 font-mono text-[11px] text-red-700 dark:text-red-300"
							>{runError}</pre
						>
					{/if}
					{#if runState === 'success'}
						<div
							class="mt-1 flex items-center justify-between gap-3 rounded-md border border-green-300 bg-green-50 p-2 dark:border-green-800 dark:bg-green-950/40"
						>
							<span class="text-xs text-green-900 dark:text-green-100">
								Looks good? Save this run as the Hub recording.
							</span>
							<Button
								size="xs"
								variant="accent"
								startIcon={{ icon: Check }}
								onclick={saveRecording}
							>
								Save as recording
							</Button>
						</div>
					{:else if runState === 'failed'}
						<span class="text-[11px] text-hint">
							Fix inputs and try again. Only successful runs can be saved as a recording.
						</span>
					{/if}
				</div>
			{/if}

			{#if recordSchemaLoading}
				<span class="text-xs text-hint">Loading schema…</span>
			{:else}
				<SchemaForm bind:args={recordArgs} bind:isValid={recordValid} schema={recordSchema} />
			{/if}
		</div>
		{#snippet actions()}
			{#if runState === 'success'}
				<Button variant="default" startIcon={{ icon: RotateCcw }} onclick={runJob}>Re-run</Button>
				<Button variant="accent" startIcon={{ icon: Check }} onclick={saveRecording}>
					Save as recording
				</Button>
			{:else}
				<Button
					variant="accent"
					loading={runState === 'running'}
					disabled={!recordValid || recordSchemaLoading}
					startIcon={{ icon: Play }}
					onclick={runJob}
				>
					{runState === 'failed' ? 'Re-run' : 'Run'}
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={publishDrawer} size="600px">
	<DrawerContent
		title={publishTarget ? `Share as iframe — ${publishTarget.path}` : 'Share as iframe'}
		on:close={() => publishDrawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Expose <span class="font-mono text-emphasis">{publishTarget?.path}</span> at a public URL so
				it can be embedded as an iframe (e.g. on the Hub, a docs page, or your own site). Anyone with
				the URL will be able to interact with it.
			</p>

			<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
				<div class="flex items-center gap-2">
					<TriangleAlert
						size={14}
						class={workspaceRateLimit ? 'text-secondary' : 'text-orange-600 dark:text-orange-400'}
					/>
					<span class="text-sm font-semibold">Rate limit (workspace-wide)</span>
					<Tooltip>
						Caps public app executions per minute per server. Applies to all public apps in this
						workspace.
					</Tooltip>
				</div>
				{#if workspaceRateLimit && workspaceRateLimit > 0}
					<span class="text-xs text-secondary">
						Currently <span class="font-mono text-emphasis">{workspaceRateLimit}</span> executions /
						minute / server.
					</span>
				{:else}
					<span class="text-xs text-orange-700 dark:text-orange-300">
						No rate limit configured — anyone with the URL can hit this app at any rate.
					</span>
				{/if}
				<a
					href="{base}/workspace_settings?tab=default_app"
					class="text-[11px] text-blue-600 underline"
					onclick={() => publishDrawer?.closeDrawer()}
				>
					Edit in Workspace settings → Apps
				</a>
			</div>
		</div>
		{#snippet actions()}
			<Button variant="default" onclick={() => publishDrawer?.closeDrawer()}>Cancel</Button>
			<Button
				variant="accent"
				loading={publishing}
				startIcon={{ icon: Globe }}
				onclick={confirmPublish}
			>
				Generate iframe
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={resourceDrawer} size="640px">
	<DrawerContent title="Resource dependencies" on:close={() => resourceDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Resource types the selected items depend on. Each is synced to the Hub so a fork knows what
				credentials it needs to fill. <span class="font-semibold">Input</span> means the item takes
				the resource as a parameter; <span class="font-semibold">hardcoded path</span> means the item
				pins a specific resource path in its code.
			</p>
			{#if dependencyTypes.length === 0}
				<span class="text-xs text-hint">No resource references in the current selection.</span>
			{:else}
				{#each dependencyTypes as r (r.resource_type)}
					<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
						<div class="flex items-center gap-2 border-b pb-2">
							<span
								class="rounded border px-1.5 py-0.5 font-mono text-xs text-primary {r.hasHardcoded
									? 'border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40'
									: 'bg-surface'}"
							>
								{r.resource_type}
							</span>
							<span class="text-[11px] text-hint">
								{r.usages.length} usage{r.usages.length > 1 ? 's' : ''}
							</span>
						</div>
						<div class="flex flex-col gap-3">
							{#each r.usages as u, ui (ui)}
								{#if u.role === 'trigger'}
									<div class="flex items-center gap-2 text-xs">
										<Zap size={14} class="shrink-0 text-hint" />
										<span class="break-all font-mono text-primary">{u.label}</span>
										<span
											class="ml-auto inline-flex shrink-0 items-center gap-1 rounded bg-surface px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide text-secondary"
										>
											{TRIGGER_KINDS[u.triggerKind].badge} trigger
										</span>
									</div>
								{:else}
									{@const itemUrl = openItemUrl(u.kind, u.itemPath)}
									<div class="flex flex-col gap-1 text-xs">
										<div class="flex items-center gap-2">
											{#if u.kind === 'script'}
												<Code2 size={14} class="shrink-0 text-hint" />
											{:else if u.kind === 'flow'}
												<BarsStaggered size={14} class="shrink-0 text-hint" />
											{:else}
												<LayoutDashboard size={14} class="shrink-0 text-hint" />
											{/if}
											<span class="break-all font-mono text-primary">{u.label}</span>
											<span
												class="ml-auto inline-flex shrink-0 items-center gap-1 rounded px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide {u.role ===
												'hardcoded'
													? 'bg-amber-100 text-amber-800 dark:bg-amber-950/50 dark:text-amber-200'
													: 'bg-surface text-secondary'}"
											>
												{u.role === 'hardcoded' ? 'hardcoded path' : 'input'}
												{#if u.role === 'hardcoded'}
													<Popover notClickable placement="top">
														<Info size={11} class="text-amber-600 dark:text-amber-400" />
														{#snippet text()}
															<div
																class="flex w-80 max-w-[90vw] flex-col gap-2 text-left text-[11px] normal-case"
															>
																<span>
																	This {u.kind} references the resource by a hardcoded path
																	<span class="break-all font-mono">$res:{u.path}</span>.
																</span>
																<span class="text-hint">
																	For portability, prefer taking the resource as an input — a fork
																	won't have this exact path. It's relocated into the project on
																	publish, but converting it to an input keeps the item reusable.
																</span>
															</div>
														{/snippet}
													</Popover>
												{/if}
											</span>
											{#if itemUrl}
												<a
													href={itemUrl}
													target="_blank"
													rel="noopener"
													title="Open {u.kind} in new tab"
													class="shrink-0 text-hint hover:text-primary"
												>
													<ExternalLink size={12} />
												</a>
											{/if}
										</div>
									</div>
								{/if}
							{/each}
						</div>
					</div>
				{/each}
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={triggerDrawer} size="640px">
	<DrawerContent title="Triggers" on:close={() => triggerDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Triggers attached to the selected scripts and flows. Synced to the Hub as
				<span class="font-semibold">disabled stubs</span>. Recipients review and enable each one
				manually after importing. External hooks (Slack/Discord webhooks, message-queue
				subscriptions, etc.) must be re-registered against the importing instance.
			</p>
			{#if relevantTriggers.length === 0}
				<span class="text-xs text-hint">No triggers reference the selected items.</span>
			{:else}
				{#each triggersByKind as [kind, triggers] (kind)}
					<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
						<div class="flex items-center gap-2 border-b pb-2">
							<span class="rounded border bg-surface px-1.5 py-0.5 font-mono text-xs text-primary">
								{TRIGGER_KINDS[kind].badge}
							</span>
							<span class="text-[11px] text-hint">
								{triggers.length} trigger{triggers.length > 1 ? 's' : ''}
							</span>
							{#if TRIGGER_KINDS[kind].note}
								<span class="ml-auto">
									<Popover notClickable placement="top">
										<Info size={12} class="text-blue-600 dark:text-blue-400" />
										{#snippet text()}
											<div
												class="flex w-72 max-w-[90vw] flex-col gap-1 text-left text-[11px] normal-case"
											>
												<span>{TRIGGER_KINDS[kind].note}</span>
											</div>
										{/snippet}
									</Popover>
								</span>
							{/if}
						</div>
						<div class="flex flex-col gap-3">
							{#each triggers as t (t.path)}
								{@const runnableSummary = runnableSummaryByPath.get(
									`${t.is_flow ? 'flow' : 'script'}:${t.script_path}`
								)}
								{@const details = triggerDetails(t)}
								{@const cfg = t.config as any}
								{@const previewKey = t.kind === 'schedule' ? `${cfg.schedule}|${cfg.timezone}` : ''}
								{@const preview = t.kind === 'schedule' ? schedulePreviews[previewKey] : undefined}
								{@const triggerUrl = openTriggerUrl(t.kind)}
								<div class="flex flex-col gap-1 text-xs">
									<div class="flex items-center gap-2">
										{#if t.is_flow}
											<BarsStaggered size={14} class="shrink-0 text-hint" />
										{:else}
											<Code2 size={14} class="shrink-0 text-hint" />
										{/if}
										<span class="break-all font-mono text-primary">
											{runnableSummary || t.script_path}
										</span>
										<span
											class="ml-auto inline-flex shrink-0 items-center gap-1 rounded bg-surface px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide text-secondary"
										>
											{t.is_flow ? 'flow' : 'script'}
										</span>
										{#if triggerUrl}
											<a
												href={triggerUrl}
												target="_blank"
												rel="noopener"
												title="Open {TRIGGER_KINDS[t.kind].badge} trigger list in new tab"
												class="shrink-0 text-hint hover:text-primary"
											>
												<ExternalLink size={12} />
											</a>
										{/if}
									</div>
									{#if details.length > 0}
										<dl class="ml-5 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5 text-[11px]">
											{#each details as d (d.label)}
												<dt class="text-hint">{d.label}</dt>
												<dd class="break-all font-mono text-tertiary">{d.value}</dd>
											{/each}
											{#if t.kind === 'schedule'}
												<dt class="text-hint">Next runs</dt>
												<dd class="text-tertiary">
													{#if preview && preview.length > 0}
														<div class="flex flex-col gap-0.5">
															{#each preview as date (date)}
																<span>{displayDate(date)}</span>
															{/each}
														</div>
													{:else if preview && preview.length === 0}
														<span class="text-hint">No upcoming run</span>
													{:else}
														<span class="text-hint">Loading…</span>
													{/if}
												</dd>
											{/if}
										</dl>
									{/if}
								</div>
							{/each}
						</div>
					</div>
				{/each}
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={bundleDrawer} size="600px">
	<DrawerContent title="Bundle to Hub" on:close={() => bundleDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Name and document your bundle. The readme can be updated later, but a clear one speeds up
				the Windmill team's review.
			</p>
			<label class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Name</span>
				<TextInput bind:value={hubName} inputProps={{ placeholder: 'e.g. Acme CRM toolkit' }} />
			</label>
			<div class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Project slug</span>
				<span class="rounded border bg-surface-secondary px-2 py-1.5 font-mono text-secondary">
					{effectiveSlug || sanitizeSlug(hubName) || '—'}
				</span>
				<span class="text-[11px] text-hint">
					{#if effectiveSlug}
						Locked — items live under <span class="font-mono">f/{effectiveSlug}/</span>.
					{:else if hubName.trim() && !isValidSlug(sanitizeSlug(hubName))}
						<span class="text-red-600 dark:text-red-400">
							The name yields an invalid slug. Use at least 3 letters/digits.
						</span>
					{:else}
						Auto-generated from the name. Once project forked, items will live under
						<span class="font-mono">f/{sanitizeSlug(hubName) || '<slug>'}/</span>.
					{/if}
				</span>
			</div>
			<label class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Summary</span>
				<TextInput
					bind:value={hubSummary}
					inputProps={{ placeholder: 'Short one-liner shown on the Hub card' }}
				/>
			</label>
			<label class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Readme</span>
				<textarea
					bind:value={hubReadme}
					placeholder={"# What this workspace does\n\n# Who it's for\n\n# How to use it\n"}
					rows="10"
					class="rounded border px-2 py-1.5 text-xs font-mono bg-surface"
				></textarea>
				<span class="text-[11px] text-hint">
					Markdown supported. Editable any time before and after publication.
				</span>
			</label>
			<div class="flex flex-col gap-2 border-t pt-4 text-xs">
				<div class="flex items-center gap-2">
					<Database size={14} />
					<span class="font-semibold text-primary">Data table migrations</span>
				</div>
				{#if migrationsGenerating}
					<div class="flex items-center gap-2 text-secondary">
						<Loader2 size={14} class="animate-spin" />
						Detecting data tables used by this project…
					</div>
				{:else if migrationDrafts.length === 0}
					<span class="text-[11px] text-hint">
						No data table usage detected in this project's scripts, flows, or raw apps.
					</span>
				{:else}
					<span class="text-[11px] text-hint">
						We detected these data tables. When included, the migration recreates their tables on
						import. Best-effort — review and edit before publishing.
					</span>
					{#each migrationDrafts as m (m.datatable_name)}
						<div class="flex flex-col gap-1.5 rounded border bg-surface-secondary p-2">
							<div class="flex items-center justify-between gap-2">
								<span class="font-mono text-primary">{m.datatable_name}</span>
								<Toggle bind:checked={m.enabled} size="xs" options={{ right: 'Include' }} />
							</div>
							<!-- Always shown: a disabled entry may carry `--` comments explaining
							     what couldn't be auto-generated, which the user edits then includes. -->
							<textarea
								bind:value={m.sql}
								rows="6"
								spellcheck="false"
								placeholder={`-- SQL migration for ${m.datatable_name}`}
								class="rounded border bg-surface px-2 py-1.5 text-[11px] font-mono"
							></textarea>
						</div>
					{/each}
				{/if}
			</div>
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				loading={deploying}
				disabled={!hubName.trim() || (!effectiveSlug && !isValidSlug(sanitizeSlug(hubName)))}
				startIcon={{ icon: Cloud }}
				onclick={confirmBundle}
			>
				Create bundle
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

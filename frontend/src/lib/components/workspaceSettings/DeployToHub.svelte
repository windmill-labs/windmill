<script lang="ts">
	// Hub deploy + share-as-iframe surface for the workspace.
	// - Items list is fetched live from the workspace (apps, raw_apps, flows, scripts, resources).
	// - Rate limit is read live from WorkspaceService.getSettings.
	// - Share-as-iframe flips the app's execution_mode to 'anonymous' via AppService.updateApp
	//   and resolves the real secret URL via AppService.getPublicSecretOfApp.
	// - The "Deploy to Hub" bundle/version flow is still mocked: no backend endpoint exists yet.
	// - Recording per script/flow is also mocked for the same reason.
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import WorkspaceDeployLayout from '$lib/components/WorkspaceDeployLayout.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		AppService,
		FlowService,
		JobService,
		RawAppService,
		ResourceService,
		ScriptService,
		WorkspaceService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { computeSecretUrl } from '$lib/components/apps/editor/appDeploy.svelte'
	import {
		Check,
		Cloud,
		Copy,
		ExternalLink,
		Globe,
		Loader2,
		Play,
		RotateCcw,
		TriangleAlert,
		X
	} from 'lucide-svelte'
	import type { Kind } from '$lib/utils_deployable'

	type Phase = 'predeploy' | 'live'
	type RecStatus = 'none' | 'recording' | 'recorded'
	interface DeployItem {
		key: string
		path: string
		kind: Kind
		summary?: string
		resourceType?: string
		rec: RecStatus
		published?: boolean
		publicUrl?: string
		[k: string]: unknown
	}

	const canRecord = (k: Kind) => k === 'script' || k === 'flow'
	const canPublishApp = (k: Kind) => k === 'app' || k === 'raw_app'

	let items = $state<DeployItem[]>([])
	let loading = $state(false)
	let workspaceRateLimit = $state<number | undefined>(undefined)

	const EMPTY_SCHEMA = { type: 'object', properties: {}, required: [] }
	// MOCK: no backend endpoint exposes a hub-slug or hub version per workspace yet.
	let hubSlug = $derived($workspaceStore ?? '')
	let hubUrl = $derived(`https://hub.windmill.dev/workspaces/${hubSlug}`)

	let phase = $state<Phase>('predeploy')
	let hubVersion = $state<number>(0)
	let deploymentStatus = $state<
		Record<string, { status: 'loading' | 'deployed' | 'failed'; error?: string }>
	>({})
	let deploying = $state(false)

	let recordDrawer = $state<Drawer | undefined>()
	let recordTarget = $state<DeployItem | undefined>()
	let recordArgs = $state<Record<string, any>>({})
	let recordValid = $state(true)
	let recordSchema = $state<Record<string, any>>(EMPTY_SCHEMA)
	let recordSchemaLoading = $state(false)
	type RunState = 'idle' | 'running' | 'success' | 'failed'
	let runState = $state<RunState>('idle')
	let runJobId = $state<string | undefined>(undefined)
	let runResult = $state<unknown>(undefined)
	let runError = $state<string | undefined>(undefined)
	// MOCK STORAGE: backend has no recordings table yet; we keep the job_id per item locally.
	let recordings = $state<Record<string, string>>({})

	let publishDrawer = $state<Drawer | undefined>()
	let publishTarget = $state<DeployItem | undefined>()
	let publishing = $state(false)

	const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

	async function loadWorkspace(workspace: string) {
		loading = true
		try {
			const [apps, rawApps, flows, scripts, resources, settings] = await Promise.all([
				AppService.listApps({ workspace, perPage: 100 }),
				RawAppService.listRawApps({ workspace, perPage: 100 }),
				FlowService.listFlows({ workspace, perPage: 100 }),
				ScriptService.listScripts({ workspace, perPage: 100 }),
				ResourceService.listResource({ workspace, perPage: 100 }),
				WorkspaceService.getSettings({ workspace }).catch(() => undefined)
			])

			workspaceRateLimit = settings?.public_app_execution_limit_per_minute

			const next: DeployItem[] = []
			for (const a of apps) {
				const isPublic = a.execution_mode === 'anonymous'
				next.push({
					key: `app:${a.path}`,
					path: a.path,
					kind: 'app',
					summary: a.summary,
					rec: 'none',
					published: isPublic,
					publicUrl: isPublic ? await resolvePublicUrl(workspace, a.path) : undefined
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
			// Mirrors backend/workspaces_export.rs: skip internal types not meant for export.
			const HIDDEN_RESOURCE_TYPES = new Set(['app_theme', 'state', 'cache'])
			for (const r of resources) {
				if (HIDDEN_RESOURCE_TYPES.has(r.resource_type)) continue
				next.push({
					key: `resource:${r.path}`,
					path: r.path,
					kind: 'resource',
					resourceType: r.resource_type,
					rec: 'none'
				})
			}
			items = next
		} catch (e: any) {
			sendUserToast(`Failed to load workspace items: ${e?.message ?? e}`, true)
		} finally {
			loading = false
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

	$effect(() => {
		if ($workspaceStore) {
			loadWorkspace($workspaceStore)
		}
	})

	async function deployAll() {
		// MOCK: bundle/version push to the Hub is not implemented backend-side.
		deploying = true
		try {
			for (const it of items) {
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'loading' } }
				await delay(120)
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'deployed' } }
			}
			await delay(150)
			hubVersion += 1
			if (phase === 'live') {
				items = items.map((i) => ({ ...i, rec: i.rec === 'recorded' ? 'none' : i.rec }))
			}
			deploymentStatus = {}
			phase = 'live'
			sendUserToast(
				hubVersion === 1
					? `Published to the Hub as v1 (${items.length} items)`
					: `Republished to the Hub as v${hubVersion}`
			)
		} finally {
			deploying = false
		}
	}

	async function openRecord(it: DeployItem) {
		recordTarget = it
		recordArgs = {}
		recordValid = true
		recordSchema = EMPTY_SCHEMA
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
				recordSchema = (s.schema as Record<string, any>) ?? EMPTY_SCHEMA
			} else if (it.kind === 'flow') {
				const f = await FlowService.getFlowByPath({ workspace, path: it.path })
				recordSchema = (f.schema as Record<string, any>) ?? EMPTY_SCHEMA
			}
		} catch (e: any) {
			sendUserToast(`Failed to load schema: ${e?.message ?? e}`, true)
		} finally {
			recordSchemaLoading = false
		}
	}
	async function runJob() {
		const it = recordTarget
		const workspace = $workspaceStore
		if (!it || !workspace) return
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
				runState = 'idle'
				return
			}
			runJobId = jobId
			await pollJobUntilComplete(workspace, jobId)
		} catch (e: any) {
			runState = 'failed'
			runError = `Failed to start: ${e?.message ?? e}`
		}
	}
	async function pollJobUntilComplete(workspace: string, jobId: string) {
		for (let i = 0; i < 300; i++) {
			await delay(1000)
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace,
					id: jobId
				})
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
				runState = 'failed'
				runError = `Polling failed: ${e?.message ?? e}`
				return
			}
		}
		runState = 'failed'
		runError = 'Timed out after 5 minutes'
	}
	function saveRecording() {
		const it = recordTarget
		if (!it || !runJobId || runState !== 'success') return
		// MOCK STORAGE: would persist {item_path, hub_version, job_id} server-side.
		recordings = { ...recordings, [it.key]: runJobId }
		items = items.map((i) => (i.key === it.key ? { ...i, rec: 'recorded' } : i))
		sendUserToast(`Recording saved — job ${runJobId}`)
		recordDrawer?.closeDrawer()
	}

	function openPublish(it: DeployItem) {
		publishTarget = it
		publishDrawer?.openDrawer()
	}
	async function confirmPublish() {
		const it = publishTarget
		const workspace = $workspaceStore
		if (!it || !workspace || it.kind !== 'app') return
		publishing = true
		try {
			const app = await AppService.getAppByPath({ workspace, path: it.path })
			const policy = { ...(app.policy ?? {}), execution_mode: 'anonymous' as const }
			await AppService.updateApp({
				workspace,
				path: it.path,
				requestBody: { policy, deployment_message: 'Share as iframe' }
			})
			const url = await resolvePublicUrl(workspace, it.path)
			items = items.map((i) => (i.key === it.key ? { ...i, published: true, publicUrl: url } : i))
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
		if (!workspace || it.kind !== 'app') return
		try {
			const app = await AppService.getAppByPath({ workspace, path: it.path })
			const policy = { ...(app.policy ?? {}), execution_mode: 'publisher' as const }
			await AppService.updateApp({
				workspace,
				path: it.path,
				requestBody: { policy, deployment_message: 'Unshare iframe' }
			})
			items = items.map((i) =>
				i.key === it.key ? { ...i, published: false, publicUrl: undefined } : i
			)
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
		selectedItems={[]}
		{deploymentStatus}
		hideSelection
		emptyMessage={loading ? 'Loading workspace items…' : 'No items to publish'}
	>
		{#snippet header()}
			<div class="flex flex-col gap-2 w-full pb-4">
				<div class="flex flex-wrap items-center gap-2">
					{#if phase === 'predeploy'}
						<Badge color="gray" size="xs">Not on the Hub yet</Badge>
					{:else}
						<Badge color="transparent" class="font-semibold">
							<Cloud size={14} class="mr-1" />
							<span class="text-secondary">on Hub:</span>
							<span class="text-emphasis">{hubSlug}</span>
						</Badge>
						<Badge color="blue" size="xs">v{hubVersion}</Badge>
						<a
							href={hubUrl}
							target="_blank"
							rel="noopener noreferrer"
							class="inline-flex items-center gap-1 text-xs text-blue-600 hover:underline"
						>
							<ExternalLink size={12} /> Open in Hub
						</a>
					{/if}
				</div>
				{#if phase === 'live'}
					<div
						class="flex items-start gap-2 rounded-md border bg-surface-secondary p-3 text-xs text-secondary"
					>
						<Play size={14} class="mt-0.5 shrink-0 text-blue-600" />
						<div class="flex flex-col gap-1">
							<span class="font-semibold text-primary">What is a recording?</span>
							<span>
								A recording is one real execution of a script or flow, captured with its inputs,
								logs, step outputs, and final result. Hub visitors can replay it step-by-step to
								understand how the item works without needing access to your workspace.
							</span>
						</div>
					</div>
				{/if}
			</div>
		{/snippet}

		{#snippet itemSummary(item)}
			{@const it = item as DeployItem}
			<span class="truncate">
				{it.kind === 'resource' ? (it.resourceType ?? it.path) : it.summary?.trim() || it.path}
			</span>
		{/snippet}

		{#snippet itemActions(item)}
			{@const it = item as DeployItem}
			{#if phase === 'live' && canRecord(it.kind)}
				{#if it.rec === 'recorded'}
					<Badge color="green" size="xs">
						<Check size={10} class="mr-0.5" />Recorded v{hubVersion}
					</Badge>
					{#if recordings[it.key]}
						<a
							href={`/run/${recordings[it.key]}?workspace=${$workspaceStore}`}
							target="_blank"
							rel="noopener noreferrer"
							class="inline-flex items-center gap-1 text-xs text-blue-600 hover:underline"
						>
							<ExternalLink size={12} /> Job
						</a>
					{/if}
					<Button
						size="xs"
						variant="subtle"
						startIcon={{ icon: RotateCcw }}
						onclick={() => openRecord(it)}
					>
						Re-record
					</Button>
				{:else}
					<Button
						size="xs"
						variant="subtle"
						loading={it.rec === 'recording'}
						startIcon={{ icon: Play }}
						onclick={() => openRecord(it)}
					>
						Add recording
					</Button>
				{/if}
			{/if}
			{#if canPublishApp(it.kind)}
				{#if it.published && it.publicUrl}
					<Badge color="green" size="xs">
						<Globe size={10} class="mr-0.5" />Public
					</Badge>
					<a
						href={it.publicUrl}
						target="_blank"
						rel="noopener noreferrer"
						class="inline-flex items-center gap-1 text-xs text-blue-600 hover:underline"
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
					{#if it.kind === 'app'}
						<Button size="xs" variant="subtle" onclick={() => unpublishApp(it)}>Unpublish</Button>
					{/if}
				{:else if it.kind === 'app'}
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
						Bundles the whole workspace and publishes it to the Hub as v1.
					</span>
					<Button
						variant="accent"
						loading={deploying}
						disabled={items.length === 0}
						startIcon={{ icon: Cloud }}
						onclick={deployAll}
					>
						Deploy to Hub ({items.length})
					</Button>
				{:else}
					<span class="text-[11px] text-hint">
						Republishes the current workspace state as v{hubVersion + 1}, overriding the Hub copy.
					</span>
					<Button
						variant="accent"
						loading={deploying}
						startIcon={{ icon: Cloud }}
						onclick={deployAll}
					>
						Update Hub (v{hubVersion} → v{hubVersion + 1})
					</Button>
				{/if}
			</div>
		{/snippet}
	</WorkspaceDeployLayout>
</div>

<Drawer bind:this={recordDrawer} size="600px">
	<DrawerContent
		title={recordTarget ? `Record — ${recordTarget.path}` : 'Record'}
		on:close={() => recordDrawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-3">
			<p class="text-xs text-secondary">
				Run this {recordTarget?.kind} once with the inputs below. The full execution — args, logs, intermediate
				step outputs and final result — is saved as a <b>replayable recording</b> shown on the Hub
				page for v{hubVersion}. Visitors can step through it to see how the {recordTarget?.kind} works
				without running anything themselves.
			</p>

			{#if runState !== 'idle'}
				<div
					class="sticky top-0 z-10 flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 shadow-sm"
				>
					<div class="flex items-center gap-2 text-sm">
						{#if runState === 'running'}
							<Loader2 size={14} class="animate-spin text-blue-600" />
							<span class="font-semibold">Running…</span>
						{:else if runState === 'success'}
							<Check size={14} class="text-green-600" />
							<span class="font-semibold text-green-700">Execution succeeded</span>
						{:else}
							<X size={14} class="text-red-600" />
							<span class="font-semibold text-red-700">Execution failed</span>
						{/if}
						{#if runJobId}
							<a
								href={`/run/${runJobId}?workspace=${$workspaceStore}`}
								target="_blank"
								rel="noopener noreferrer"
								class="inline-flex items-center gap-1 text-xs text-blue-600 hover:underline"
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
							class="max-h-40 overflow-auto rounded bg-surface p-2 font-mono text-[11px] text-red-700"
							>{runError}</pre
						>
					{/if}
					{#if runState === 'success'}
						<div
							class="mt-1 flex items-center justify-between gap-3 rounded-md border border-green-300 bg-green-50 p-2"
						>
							<span class="text-xs text-green-900">
								Looks good? Save this run as the Hub recording for v{hubVersion}.
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
						class={workspaceRateLimit ? 'text-secondary' : 'text-orange-600'}
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
					<span class="text-xs text-orange-700">
						No rate limit configured — anyone with the URL can hit this app at any rate.
					</span>
				{/if}
				<a
					href="#default_app"
					class="text-[11px] text-blue-600 underline"
					onclick={(e) => {
						e.preventDefault()
						publishDrawer?.closeDrawer()
						window.location.hash = 'default_app'
					}}
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

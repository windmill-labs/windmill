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
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
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

	type Phase = 'predeploy' | 'draft' | 'under_review' | 'live'
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

	let phase = $state<Phase>('predeploy')
	// Live workspace items (refreshed from API). Used in predeploy phase only.
	let workspaceItems = $state<DeployItem[]>([])
	// Snapshot frozen at deploy time. Used in draft / under_review / live phases.
	let draftItems = $state<DeployItem[]>([])
	let selectedFolders = $state<string[]>([])
	let availableFolders = $derived(
		Array.from(
			new Set(
				workspaceItems
					.map((i) => i.path.split('/').slice(0, 2).join('/'))
					.filter((p) => p.startsWith('f/') || p.startsWith('u/'))
			)
		).sort()
	)
	let filteredWorkspaceItems = $derived(
		selectedFolders.length === 0
			? workspaceItems
			: workspaceItems.filter((i) => selectedFolders.some((f) => i.path.startsWith(f + '/')))
	)
	let items = $derived(phase === 'predeploy' ? filteredWorkspaceItems : draftItems)
	let loading = $state(false)
	let workspaceRateLimit = $state<number | undefined>(undefined)
	let recordableItems = $derived(items.filter((i) => canRecord(i.kind)))
	let allRecorded = $derived(
		recordableItems.length > 0 && recordableItems.every((i) => i.rec === 'recorded')
	)

	const EMPTY_SCHEMA = { type: 'object', properties: {}, required: [] }
	// MOCK: no backend endpoint exposes a hub-slug or hub version per workspace yet.
	let hubSlug = $derived($workspaceStore ?? '')
	let hubUrl = $derived(`https://hub.windmill.dev/workspaces/${hubSlug}`)

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

	let bundleDrawer = $state<Drawer | undefined>()
	let bundleName = $state('')
	let bundleReadme = $state('')
	// MOCK: would be persisted with the Hub draft.
	let hubName = $state('')
	let hubReadme = $state('')

	const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

	function patchItem(key: string, patch: Partial<DeployItem>) {
		workspaceItems = workspaceItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
		draftItems = draftItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
	}

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
			workspaceItems = next
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

	function openBundle() {
		const folderShort =
			selectedFolders.length === 1 ? selectedFolders[0].split('/').slice(1).join('/') : ''
		bundleName = hubName || folderShort || ($workspaceStore ?? '')
		bundleReadme = hubReadme
		bundleDrawer?.openDrawer()
	}
	async function confirmBundle() {
		hubName = bundleName.trim()
		hubReadme = bundleReadme.trim()
		bundleDrawer?.closeDrawer()
		await deployAll()
	}
	async function deployAll() {
		// MOCK: bundle/version push to the Hub is not implemented backend-side.
		deploying = true
		try {
			for (const it of filteredWorkspaceItems) {
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'loading' } }
				await delay(120)
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'deployed' } }
			}
			await delay(150)
			deploymentStatus = {}
			// Freeze the set of items for this draft cycle. Workspace changes after this point
			// will not affect the draft until a new draft cycle is started.
			draftItems = filteredWorkspaceItems.map((i) => ({ ...i, rec: 'none' }))
			recordings = {}
			phase = 'draft'
			sendUserToast(
				hubVersion === 0
					? `Draft created on the Hub. Add recordings before submitting for review.`
					: `New draft created (will become v${hubVersion + 1} after review).`
			)
		} finally {
			deploying = false
		}
	}

	let submitting = $state(false)
	async function submitForReview() {
		// MOCK: no review queue backend endpoint exists.
		submitting = true
		try {
			await delay(400)
			phase = 'under_review'
			sendUserToast('Submitted for review by the Windmill team.')
		} finally {
			submitting = false
		}
	}
	async function withdrawSubmission() {
		// MOCK
		await delay(200)
		phase = 'draft'
		sendUserToast('Submission withdrawn. Draft is editable again.')
	}
	async function approve() {
		// MOCK: dev-only shortcut to simulate Windmill team approval.
		await delay(200)
		hubVersion += 1
		phase = 'live'
		sendUserToast(`Approved — published as v${hubVersion} on the Hub.`)
	}
	async function startNewDraft() {
		// MOCK: re-snapshot from the live workspace and start a fresh draft cycle.
		draftItems = filteredWorkspaceItems.map((i) => ({ ...i, rec: 'none' }))
		recordings = {}
		phase = 'draft'
		sendUserToast(`New draft started (will become v${hubVersion + 1}).`)
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
		patchItem(it.key, { rec: 'recorded' })
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
			patchItem(it.key, { published: true, publicUrl: url })
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
		selectedItems={[]}
		{deploymentStatus}
		hideSelection
		emptyMessage={loading ? 'Loading workspace items…' : 'No items to publish'}
	>
		{#snippet header()}
			{@const stepNum =
				phase === 'predeploy' ? 1 : phase === 'draft' ? 2 : phase === 'under_review' ? 3 : 4}
			<div class="flex flex-col gap-2 w-full pb-4">
				<ol
					class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 text-xs text-secondary"
				>
					<span class="text-sm font-semibold text-primary">
						How to publish your workspace to the Hub
					</span>
					<li class={stepNum === 1 ? 'text-primary' : stepNum > 1 ? 'opacity-60' : ''}>
						<span class="font-mono text-emphasis">{stepNum > 1 ? '✓' : '1.'}</span>
						<span class="font-semibold text-primary">Bundle your workspace</span> — pack every script,
						flow, app and resource into a single draft.
					</li>
					<li class={stepNum === 2 ? 'text-primary' : stepNum > 2 ? 'opacity-60' : 'opacity-40'}>
						<span class="font-mono text-emphasis">{stepNum > 2 ? '✓' : '2.'}</span>
						<span class="font-semibold text-primary">Generate iframes &amp; recordings</span> — share
						public apps as iframes and capture one execution per script/flow.
					</li>
					<li class={stepNum === 3 ? 'text-primary' : stepNum > 3 ? 'opacity-60' : 'opacity-40'}>
						<span class="font-mono text-emphasis">{stepNum > 3 ? '✓' : '3.'}</span>
						<span class="font-semibold text-primary">Submit for review</span> — send the bundle to the
						Windmill team for approval.
					</li>
				</ol>
				<div class="flex flex-wrap items-center gap-2 pt-4">
					{#if phase === 'predeploy'}
						<span class="text-sm font-semibold text-primary"> Step 1: Bundle your workspace </span>
					{:else if phase === 'draft'}
						<span class="text-sm font-semibold text-primary">
							Step 2: Generate iframes &amp; recordings
						</span>
					{:else if phase === 'under_review'}
						<span class="text-sm font-semibold text-primary">Step 3: Awaiting review</span>
					{:else}
						<span class="text-sm font-semibold text-primary">Live on the Hub — v{hubVersion}</span>
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
				</div>
				{#if phase === 'predeploy' && availableFolders.length > 0}
					<div class="flex flex-col gap-1 text-xs">
						<span class="font-semibold text-primary">Filter by folder (optional)</span>
						<MultiSelect
							bind:value={selectedFolders}
							items={availableFolders.map((f) => ({ value: f, label: f }))}
							placeholder="All folders"
						/>
						<span class="text-[11px] text-hint">
							{selectedFolders.length === 0
								? `Bundling the whole workspace — ${filteredWorkspaceItems.length} items.`
								: `Bundling ${filteredWorkspaceItems.length} items from ${selectedFolders.length} folder${selectedFolders.length > 1 ? 's' : ''}.`}
						</span>
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
					<div
						class="flex items-center gap-3 rounded-md border {allRecorded
							? 'border-green-300 bg-green-50 dark:border-green-800 dark:bg-green-950/40'
							: 'border-yellow-300 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950/40'} p-3 text-xs"
					>
						<span
							class="font-mono text-sm {allRecorded
								? 'text-green-900 dark:text-green-100'
								: 'text-yellow-900 dark:text-yellow-100'}"
						>
							{recordedCount}/{recordableItems.length}
						</span>
						<div
							class="h-1.5 flex-1 overflow-hidden rounded {allRecorded
								? 'bg-green-200 dark:bg-green-900/60'
								: 'bg-yellow-200 dark:bg-yellow-900/60'}"
						>
							<div
								class="h-full {allRecorded ? 'bg-green-500' : 'bg-yellow-500'} transition-all"
								style="width: {pct}%"
							></div>
						</div>
						<span
							class="font-semibold {allRecorded
								? 'text-green-900 dark:text-green-100'
								: 'text-yellow-900 dark:text-yellow-100'}"
						>
							{allRecorded ? 'Ready to submit' : 'Recordings needed'}
						</span>
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
					{#if it.kind === 'app' && phase !== 'under_review'}
						<Button size="xs" variant="subtle" onclick={() => unpublishApp(it)}>Unpublish</Button>
					{/if}
				{:else if it.kind === 'app' && phase !== 'under_review'}
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
						Create a bundle with every script, flow, app and resource of your workspace.
					</span>
					<Button
						variant="accent"
						loading={deploying}
						disabled={items.length === 0}
						startIcon={{ icon: Cloud }}
						onclick={openBundle}
					>
						Bundle to Hub ({items.length})
					</Button>
				{:else if phase === 'draft'}
					<span class="text-[11px] text-hint">
						{#if allRecorded}
							All scripts and flows have a recording. Ready to submit.
						{:else}
							{recordableItems.filter((i) => i.rec === 'recorded').length} of {recordableItems.length}
							scripts/flows recorded. Add a recording to each before submitting.
						{/if}
					</span>
					<Button
						variant="accent"
						loading={submitting}
						disabled={!allRecorded}
						startIcon={{ icon: Check }}
						onclick={submitForReview}
					>
						Submit for review
					</Button>
				{:else if phase === 'under_review'}
					<span class="text-[11px] text-hint">
						Withdraw to keep editing, or wait for the team to approve.
					</span>
					<Button variant="default" onclick={withdrawSubmission}>Withdraw submission</Button>
					<Button variant="accent" startIcon={{ icon: Check }} onclick={approve}>
						Simulate approval (dev)
					</Button>
				{:else}
					<span class="text-[11px] text-hint">
						Iterate further by starting a new draft for v{hubVersion + 1}.
					</span>
					<Button variant="accent" startIcon={{ icon: RotateCcw }} onclick={startNewDraft}>
						New draft (v{hubVersion + 1})
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

<Drawer bind:this={bundleDrawer} size="600px">
	<DrawerContent title="Bundle to Hub" on:close={() => bundleDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Name and document your bundle. The readme can be updated later, but a clear one speeds up
				the Windmill team's review.
			</p>
			<label class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Name</span>
				<TextInput bind:value={bundleName} placeholder="e.g. Acme CRM toolkit" />
			</label>
			<label class="flex flex-col gap-1 text-xs">
				<span class="font-semibold text-primary">Readme</span>
				<textarea
					bind:value={bundleReadme}
					placeholder={"# What this workspace does\n\n# Who it's for\n\n# How to use it\n"}
					rows="10"
					class="rounded border px-2 py-1.5 text-xs font-mono bg-surface"
				></textarea>
				<span class="text-[11px] text-hint">
					Markdown supported. Editable any time before and after publication.
				</span>
			</label>
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				loading={deploying}
				disabled={!bundleName.trim()}
				startIcon={{ icon: Cloud }}
				onclick={confirmBundle}
			>
				Create bundle
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

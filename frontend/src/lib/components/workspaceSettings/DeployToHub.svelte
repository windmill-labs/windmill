<script lang="ts">
	// NOTE: mocked preview for a team demo. No backend calls.
	// Push/override model: a single "Deploy to Hub" action bundles the whole workspace
	// and republishes it as a new version on the Hub. No per-item diff / merge / drift.
	// Recording per script/flow is kept as an orthogonal feature attached to the
	// current published version.
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import WorkspaceDeployLayout from '$lib/components/WorkspaceDeployLayout.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		Check,
		Cloud,
		Copy,
		ExternalLink,
		Globe,
		Play,
		RotateCcw,
		TriangleAlert
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
		rateLimit?: RateLimitConfig
		[k: string]: unknown
	}
	interface RateLimitConfig {
		enabled: boolean
		perMinute: number
		burst: number
		perIp: boolean
	}
	const DEFAULT_RATE_LIMIT: RateLimitConfig = {
		enabled: true,
		perMinute: 60,
		burst: 10,
		perIp: true
	}
	const WORKSPACE_DEFAULT_RATE_LIMIT = { perMinute: 120, burst: 20 }

	const canRecord = (k: Kind) => k === 'script' || k === 'flow'
	const canPublishApp = (k: Kind) => k === 'app' || k === 'raw_app'

	// --- MOCK DATA -------------------------------------------------------------
	let items = $state<DeployItem[]>([
		{
			key: 'raw_app:crm/dashboard',
			path: 'crm/dashboard',
			kind: 'raw_app',
			summary: 'Sales dashboard',
			rec: 'none'
		},
		{
			key: 'raw_app:crm/onboarding',
			path: 'crm/onboarding',
			kind: 'raw_app',
			summary: 'Customer onboarding portal',
			rec: 'none'
		},
		{
			key: 'flow:crm/sync_contacts',
			path: 'crm/sync_contacts',
			kind: 'flow',
			summary: 'Sync contacts from HubSpot',
			rec: 'none'
		},
		{
			key: 'flow:crm/enrich_lead',
			path: 'crm/enrich_lead',
			kind: 'flow',
			summary: 'Enrich lead with Clearbit',
			rec: 'none'
		},
		{
			key: 'script:crm/send_slack_digest',
			path: 'crm/send_slack_digest',
			kind: 'script',
			summary: 'Daily Slack digest',
			rec: 'none'
		},
		{
			key: 'script:crm/upsert_postgres',
			path: 'crm/upsert_postgres',
			kind: 'script',
			summary: 'Upsert rows to Postgres',
			rec: 'none'
		},
		{
			key: 'resource:postgresql',
			path: 'postgresql',
			kind: 'resource',
			resourceType: 'postgresql',
			rec: 'none'
		},
		{
			key: 'resource:slack_bot',
			path: 'slack_bot',
			kind: 'resource',
			resourceType: 'slack_bot',
			rec: 'none'
		}
	])
	const FAKE_SCHEMA = {
		type: 'object',
		properties: {
			customer: { type: 'string', description: 'Customer to run against' },
			includeArchived: { type: 'boolean', description: 'Include archived rows', default: false }
		},
		required: ['customer']
	}
	const hubSlug = 'twenty-crm'
	const hubUrl = `https://hub.windmill.dev/workspaces/${hubSlug}`
	// ---------------------------------------------------------------------------

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

	let publishDrawer = $state<Drawer | undefined>()
	let publishTarget = $state<DeployItem | undefined>()
	let publishing = $state(false)
	let publishRateLimit = $state<RateLimitConfig>({ ...DEFAULT_RATE_LIMIT })

	const mockPublicUrl = (path: string) => `https://app.windmill.dev/public/${hubSlug}/${path}`

	const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

	async function deployAll() {
		deploying = true
		try {
			for (const it of items) {
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'loading' } }
				await delay(120)
				deploymentStatus = { ...deploymentStatus, [it.key]: { status: 'deployed' } }
			}
			await delay(150)
			hubVersion += 1
			// Reset recordings on republish: they were tied to the previous version.
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

	function openRecord(it: DeployItem) {
		recordTarget = it
		recordArgs = {}
		recordValid = true
		recordDrawer?.openDrawer()
	}
	async function confirmRecord() {
		const it = recordTarget
		if (!it) return
		recordDrawer?.closeDrawer()
		items = items.map((i) => (i.key === it.key ? { ...i, rec: 'recording' } : i))
		await delay(900)
		items = items.map((i) => (i.key === it.key ? { ...i, rec: 'recorded' } : i))
	}

	function openPublish(it: DeployItem) {
		publishTarget = it
		publishRateLimit = { ...(it.rateLimit ?? DEFAULT_RATE_LIMIT) }
		publishDrawer?.openDrawer()
	}
	async function confirmPublish() {
		const it = publishTarget
		if (!it) return
		publishing = true
		try {
			await delay(500)
			const rl = { ...publishRateLimit }
			items = items.map((i) =>
				i.key === it.key
					? { ...i, published: true, publicUrl: mockPublicUrl(i.path), rateLimit: rl }
					: i
			)
			sendUserToast(
				rl.enabled
					? `${it.path} is now public (${rl.perMinute} req/min, burst ${rl.burst})`
					: `${it.path} is now public (no rate limit)`
			)
			publishDrawer?.closeDrawer()
		} finally {
			publishing = false
		}
	}
	async function unpublishApp(key: string) {
		items = items.map((i) => (i.key === key ? { ...i, published: false, publicUrl: undefined } : i))
		sendUserToast('App unpublished')
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
		emptyMessage="No items to publish"
	>
		{#snippet header()}
			<div class="flex flex-col gap-2 w-full pb-4 border-b">
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
			{#if phase === 'live' && canPublishApp(it.kind)}
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
					<Button size="xs" variant="subtle" onclick={() => unpublishApp(it.key)}>Unpublish</Button>
				{:else}
					<Button
						size="xs"
						variant="subtle"
						startIcon={{ icon: Globe }}
						onclick={() => openPublish(it)}
					>
						Publish publicly
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
				Provide the inputs to run this {recordTarget?.kind} once. The run is captured as a recording
				and attached to the item's current version on the Hub.
			</p>
			<SchemaForm bind:args={recordArgs} bind:isValid={recordValid} schema={FAKE_SCHEMA} />
		</div>
		{#snippet actions()}
			<Button variant="default" onclick={() => recordDrawer?.closeDrawer()}>Cancel</Button>
			<Button
				variant="accent"
				disabled={!recordValid}
				startIcon={{ icon: Play }}
				onclick={confirmRecord}
			>
				Run & record
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={publishDrawer} size="600px">
	<DrawerContent
		title={publishTarget ? `Publish — ${publishTarget.path}` : 'Publish'}
		on:close={() => publishDrawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				This will expose <span class="font-mono text-emphasis">{publishTarget?.path}</span> at a public
				URL so the Hub can embed it as a live iframe instead of a frontend-only run. Anyone with the
				URL will be able to interact with it.
			</p>

			<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<TriangleAlert size={14} class="text-orange-600" />
						<span class="text-sm font-semibold">Rate limit</span>
						<Tooltip>
							Caps requests to this public app. Workspace default: {WORKSPACE_DEFAULT_RATE_LIMIT.perMinute}
							req/min, burst {WORKSPACE_DEFAULT_RATE_LIMIT.burst}. Override here for this app only.
						</Tooltip>
					</div>
					<Toggle bind:checked={publishRateLimit.enabled} size="xs" />
				</div>
				{#if publishRateLimit.enabled}
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1 text-xs">
							<span class="text-secondary">Requests / minute</span>
							<input
								type="number"
								min="1"
								bind:value={publishRateLimit.perMinute}
								class="rounded border px-2 py-1 text-xs"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs">
							<span class="text-secondary">Burst</span>
							<input
								type="number"
								min="0"
								bind:value={publishRateLimit.burst}
								class="rounded border px-2 py-1 text-xs"
							/>
						</label>
					</div>
					<div class="flex items-center justify-between gap-2">
						<span class="flex items-center gap-1 text-xs text-secondary">
							Apply per client IP
							<Tooltip>If off, limit is shared across all callers (global counter).</Tooltip>
						</span>
						<Toggle bind:checked={publishRateLimit.perIp} size="xs" />
					</div>
				{:else}
					<span class="text-xs text-orange-700">
						Disabled — anyone with the URL can hit this app at any rate.
					</span>
				{/if}
				<span class="text-[11px] text-hint">
					Workspace-wide defaults live in <a
						href="#default_app"
						class="underline"
						onclick={(e) => {
							e.preventDefault()
							publishDrawer?.closeDrawer()
							window.location.hash = 'default_app'
						}}>Workspace settings → Default app → Rate limiting</a
					>.
				</span>
			</div>

			{#if publishTarget}
				<div class="flex flex-col gap-1 text-xs">
					<span class="text-secondary">Public URL once published:</span>
					<code class="rounded bg-surface-secondary p-2 break-all">
						{mockPublicUrl(publishTarget.path)}
					</code>
				</div>
			{/if}
		</div>
		{#snippet actions()}
			<Button variant="default" onclick={() => publishDrawer?.closeDrawer()}>Cancel</Button>
			<Button
				variant="accent"
				loading={publishing}
				startIcon={{ icon: Globe }}
				onclick={confirmPublish}
			>
				Publish publicly
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

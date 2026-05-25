<script lang="ts">
	// NOTE: mocked preview for a team demo. No backend calls.
	// Push/override model: a single "Deploy to Hub" action bundles the whole workspace
	// and republishes it as a new version on the Hub. No per-item diff / merge / drift.
	// Recording per script/flow is kept as an orthogonal feature attached to the
	// current published version.
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import WorkspaceDeployLayout from '$lib/components/WorkspaceDeployLayout.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
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
		[k: string]: unknown
	}
	// MOCK: would be fetched via WorkspaceService.getSettings().public_app_execution_limit_per_minute
	let workspaceRateLimit = $state<number | undefined>(120)

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
		publishDrawer?.openDrawer()
	}
	async function confirmPublish() {
		const it = publishTarget
		if (!it) return
		publishing = true
		try {
			await delay(500)
			items = items.map((i) =>
				i.key === it.key ? { ...i, published: true, publicUrl: mockPublicUrl(i.path) } : i
			)
			sendUserToast(`${it.path} is now public`)
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
				Generate iframe
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

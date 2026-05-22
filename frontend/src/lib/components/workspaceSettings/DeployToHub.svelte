<script lang="ts">
	import { Button } from '$lib/components/common'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { workspaceStore } from '$lib/stores'
	import { AppService, FlowService, RawAppService, ResourceService, ScriptService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		AppWindow,
		ChevronDown,
		ChevronRight,
		Database,
		ExternalLink,
		FileCode2,
		Layout,
		Loader2,
		Workflow
	} from 'lucide-svelte'
	import { onMount } from 'svelte'

	type Bucket = 'script' | 'flow' | 'app' | 'raw_app' | 'resource'

	interface Item {
		path: string
		kind: Bucket
		summary?: string
		resourceType?: string
	}

	const BUCKET_ORDER: Bucket[] = ['app', 'raw_app', 'flow', 'script', 'resource']
	const BUCKET_LABEL: Record<Bucket, string> = {
		app: 'Apps',
		raw_app: 'Raw apps',
		flow: 'Flows',
		script: 'Scripts',
		resource: 'Resources'
	}
	const BUCKET_ICON = {
		app: AppWindow,
		raw_app: Layout,
		flow: Workflow,
		script: FileCode2,
		resource: Database
	} as const

	let items = $state<Item[]>([])
	let loadingItems = $state(false)
	let deploying = $state(false)
	let collapsed = $state<Partial<Record<Bucket, boolean>>>({})

	let grouped = $derived(
		BUCKET_ORDER.map((b) => ({ bucket: b, list: items.filter((i) => i.kind === b) })).filter(
			(g) => g.list.length > 0
		)
	)

	async function loadItems() {
		if (!$workspaceStore) return
		loadingItems = true
		const workspace = $workspaceStore
		try {
			const [scripts, flows, apps, rawApps, resources] = await Promise.all([
				ScriptService.listScripts({ workspace }),
				FlowService.listFlows({ workspace }),
				AppService.listApps({ workspace }),
				RawAppService.listRawApps({ workspace }),
				ResourceService.listResource({ workspace })
			])
			items = [
				...apps.map((a) => ({ path: a.path, kind: 'app' as const, summary: a.summary })),
				...rawApps.map((a) => ({ path: a.path, kind: 'raw_app' as const, summary: a.summary })),
				...flows.map((f) => ({ path: f.path, kind: 'flow' as const, summary: f.summary })),
				...scripts.map((s) => ({ path: s.path, kind: 'script' as const, summary: s.summary })),
				...resources
					// Exclude the auto-created app theme resources present in every workspace.
					.filter((r) => r.resource_type !== 'app_theme')
					.map((r) => ({
						path: r.path,
						kind: 'resource' as const,
						summary: r.description,
						resourceType: r.resource_type
					}))
			]
		} finally {
			loadingItems = false
		}
	}

	function toggle(b: Bucket) {
		collapsed = { ...collapsed, [b]: !collapsed[b] }
	}

	async function deployToHub() {
		if (items.length === 0) return
		deploying = true
		try {
			// TODO: build the bundle and POST it to the hub `/workspaces/add` endpoint.
			sendUserToast(`Deploying ${items.length} item(s) to the Hub`)
		} finally {
			deploying = false
		}
	}

	onMount(() => {
		loadItems()
	})
</script>

<div class="flex flex-col gap-4">
	{#if loadingItems}
		<div class="flex items-center gap-2 text-secondary text-sm">
			<Loader2 class="animate-spin" size={16} /> Loading items…
		</div>
	{:else if items.length === 0}
		<p class="text-sm text-secondary">No deployable items found in this workspace.</p>
	{:else}
		<div class="border rounded-md bg-surface-tertiary overflow-hidden">
			<div class="px-3 py-2.5 border-b text-xs font-semibold uppercase tracking-wide text-tertiary">
				Files
			</div>
			<div class="py-2 overflow-y-auto max-h-[calc(100dvh-20rem)]">
				{#each grouped as { bucket, list } (bucket)}
					{@const Icon = BUCKET_ICON[bucket]}
					{@const open = !collapsed[bucket]}
					<button
						type="button"
						class="w-full px-3 py-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider hover:bg-surface-secondary/60 transition text-tertiary hover:text-secondary"
						onclick={() => toggle(bucket)}
					>
						{#if open}
							<ChevronDown size={12} class="shrink-0" />
						{:else}
							<ChevronRight size={12} class="shrink-0" />
						{/if}
						<Icon size={12} class="shrink-0" />
						<span>{BUCKET_LABEL[bucket]}</span>
						<span class="ml-auto text-[10px] font-medium normal-case text-hint">{list.length}</span>
					</button>
					{#if open}
						<ul>
							{#each list as it (it.kind + ':' + it.path)}
								<li
									class="flex items-center gap-2 pl-8 pr-3 py-1 text-xs text-primary truncate"
									title={it.path}
								>
									{#if it.kind === 'resource'}
										{#if it.resourceType}
											<IconedResourceType
												name={it.resourceType}
												silent
												width="16px"
												height="16px"
											/>
											<span class="truncate"
												>{it.resourceType.charAt(0).toUpperCase() + it.resourceType.slice(1)}</span
											>
										{:else}
											<span class="truncate">{it.path}</span>
										{/if}
									{:else}
										<span class="truncate">{it.summary?.trim() || it.path}</span>
									{/if}
								</li>
							{/each}
						</ul>
					{/if}
				{/each}
			</div>
		</div>

		<div class="flex justify-end">
			<Button
				variant="accent"
				loading={deploying}
				startIcon={{ icon: ExternalLink }}
				onclick={deployToHub}
			>
				Deploy to Hub ({items.length})
			</Button>
		</div>
	{/if}
</div>

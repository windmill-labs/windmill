<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { workspaceStore } from '$lib/stores'
	import { AppService, FlowService, RawAppService, ResourceService, ScriptService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		AppWindow,
		Check,
		ChevronDown,
		ChevronRight,
		CircleSlash,
		Database,
		ExternalLink,
		FileCode2,
		Layout,
		Loader2,
		Play,
		RotateCcw,
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

	// Which buckets need a per-item artifact before deploy.
	// script/flow → execution recording, app/raw_app → built bundle.
	type Artifact = 'recording' | 'bundle'
	function artifactOf(kind: Bucket): Artifact | undefined {
		if (kind === 'script' || kind === 'flow') return 'recording'
		if (kind === 'app' || kind === 'raw_app') return 'bundle'
		return undefined
	}

	type ItemStatus = 'idle' | 'running' | 'done' | 'error'
	interface ItemState {
		status: ItemStatus
		durationMs?: number
	}

	const BUCKET_ORDER: Bucket[] = ['app', 'raw_app', 'flow', 'script', 'resource']
	const BUCKET_LABEL: Record<Bucket, string> = {
		app: 'Apps',
		raw_app: 'Raw apps',
		flow: 'Flows',
		script: 'Scripts',
		resource: 'Resource types'
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
	let collapsed = $state<Partial<Record<Bucket, boolean>>>({})
	let states = $state<Record<string, ItemState>>({})
	let deploying = $state(false)

	// Recording input drawer: clicking Record on a script/flow opens a form built
	// from the item's JSON schema so the user provides the args used for the run.
	let recordDrawer = $state<Drawer | undefined>()
	let recordTarget = $state<Item | undefined>()
	let recordSchema = $state<Record<string, any> | undefined>()
	let recordArgs = $state<Record<string, any>>({})
	let recordValid = $state(true)
	let loadingSchema = $state(false)

	function idOf(it: Item): string {
		return it.kind + ':' + it.path
	}

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
				// Only known/shared resource types: dedupe the resource_type of each resource,
				// excluding the auto-created app_theme and workspace-local custom types (`c_*`).
				...[
					...new Set(
						resources
							.map((r) => r.resource_type)
							.filter((rt): rt is string => !!rt && rt !== 'app_theme' && !rt.startsWith('c_'))
					)
				].map((rt) => ({ path: rt, kind: 'resource' as const, resourceType: rt }))
			]
		} finally {
			loadingItems = false
		}
	}

	let grouped = $derived(
		BUCKET_ORDER.map((b) => ({ bucket: b, list: items.filter((i) => i.kind === b) })).filter(
			(g) => g.list.length > 0
		)
	)

	// Items that require an artifact (recording or bundle) before deploy.
	let actionable = $derived(items.filter((i) => artifactOf(i.kind) !== undefined))
	let doneCount = $derived(actionable.filter((i) => states[idOf(i)]?.status === 'done').length)
	let allDone = $derived(actionable.length > 0 && doneCount === actionable.length)

	function statusOf(it: Item): ItemState {
		return states[idOf(it)] ?? { status: 'idle' }
	}

	function toggle(b: Bucket) {
		collapsed = { ...collapsed, [b]: !collapsed[b] }
	}

	// MOCK: simulate running the script/flow with `args` (recording) or building the
	// app bundle. Real impl will drive the recording stores + JobLoader.
	async function runItem(it: Item, _args: Record<string, any> = {}): Promise<void> {
		const id = idOf(it)
		states = { ...states, [id]: { status: 'running' } }
		const ms = 700 + Math.round(Math.random() * 1500)
		await new Promise((r) => setTimeout(r, ms))
		// MOCK: ~10% failure to show the error state.
		const failed = Math.random() < 0.1
		states = {
			...states,
			[id]: failed ? { status: 'error' } : { status: 'done', durationMs: ms }
		}
	}

	// Fetch the JSON schema of a script/flow so the input form can be rendered.
	async function fetchSchema(it: Item): Promise<Record<string, any> | undefined> {
		if (!$workspaceStore) return undefined
		const workspace = $workspaceStore
		if (it.kind === 'script') {
			const s = await ScriptService.getScriptByPath({ workspace, path: it.path })
			return s.schema as Record<string, any> | undefined
		}
		if (it.kind === 'flow') {
			const f = await FlowService.getFlowByPath({ workspace, path: it.path })
			return f.schema as Record<string, any> | undefined
		}
		return undefined
	}

	// Entry point for the per-item action button.
	async function startItem(it: Item): Promise<void> {
		const artifact = artifactOf(it.kind)
		if (artifact === 'bundle') {
			// Apps need no inputs — build directly.
			await runItem(it)
			return
		}
		// Recording: collect inputs first via the drawer.
		recordTarget = it
		recordArgs = {}
		recordValid = true
		recordSchema = undefined
		recordDrawer?.openDrawer()
		loadingSchema = true
		try {
			recordSchema = await fetchSchema(it)
		} finally {
			loadingSchema = false
		}
	}

	async function confirmRecord(): Promise<void> {
		const it = recordTarget
		if (!it) return
		recordDrawer?.closeDrawer()
		await runItem(it, recordArgs)
	}

	function cancelRecord(): void {
		recordDrawer?.closeDrawer()
	}

	async function deployToHub() {
		deploying = true
		try {
			// TODO: bundle (tarball export + recordings + app bundles + resource types)
			// and POST to the Hub.
			await new Promise((r) => setTimeout(r, 600))
			sendUserToast(`Submitted ${items.length} item(s) to the Hub for review`)
		} finally {
			deploying = false
		}
	}

	function actionVerb(a: Artifact): string {
		return a === 'recording' ? 'Record' : 'Build'
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
			<div
				class="px-3 py-2.5 border-b flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-tertiary"
			>
				<span>Files</span>
				<span class="ml-auto normal-case font-medium text-hint">
					{doneCount}/{actionable.length} ready
				</span>
			</div>
			<div class="py-2 overflow-y-auto max-h-[calc(100dvh-22rem)]">
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
								{@const artifact = artifactOf(it.kind)}
								{@const st = statusOf(it)}
								<li
									class="flex items-center gap-2 pl-8 pr-3 py-1 text-xs text-primary"
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
										<span class="ml-auto flex items-center gap-1 text-[10px] text-hint">
											<CircleSlash size={11} /> Referenced
										</span>
									{:else}
										<span class="truncate">{it.summary?.trim() || it.path}</span>
										{#if artifact}
											<div class="ml-auto flex items-center gap-2 shrink-0">
												{#if st.status === 'done'}
													<span class="flex items-center gap-1 text-[10px] text-green-600">
														<Check size={12} />
														{artifact === 'recording' ? 'Recorded' : 'Built'}
														{#if st.durationMs}
															<span class="text-hint">({(st.durationMs / 1000).toFixed(1)}s)</span>
														{/if}
													</span>
													<Button
														variant="subtle"
														unifiedSize="sm"
														iconOnly
														startIcon={{ icon: RotateCcw }}
														onclick={() => startItem(it)}
													/>
												{:else if st.status === 'error'}
													<span class="text-[10px] text-red-500">Failed</span>
													<Button
														variant="default"
														unifiedSize="sm"
														startIcon={{ icon: RotateCcw }}
														onclick={() => startItem(it)}
													>
														Retry
													</Button>
												{:else}
													<Button
														variant="default"
														unifiedSize="sm"
														loading={st.status === 'running'}
														startIcon={{ icon: Play }}
														onclick={() => startItem(it)}
													>
														{actionVerb(artifact)}
													</Button>
												{/if}
											</div>
										{/if}
									{/if}
								</li>
							{/each}
						</ul>
					{/if}
				{/each}
			</div>
		</div>

		<div class="flex items-center justify-end gap-3">
			{#if !allDone}
				<span class="text-[11px] text-hint">
					Record all scripts/flows and build all apps before submitting.
				</span>
			{/if}
			<Button
				variant="accent"
				loading={deploying}
				disabled={!allDone}
				startIcon={{ icon: ExternalLink }}
				onclick={deployToHub}
			>
				Submit to Hub ({items.length})
			</Button>
		</div>
	{/if}
</div>

<Drawer bind:this={recordDrawer} size="600px" on:close={cancelRecord}>
	<DrawerContent
		title={recordTarget ? `Record — ${recordTarget.path}` : 'Record'}
		on:close={cancelRecord}
	>
		<div class="flex flex-col gap-3">
			<p class="text-xs text-secondary">
				Provide the inputs to run this {recordTarget?.kind} once. The run is captured as a recording
				and shipped with the workspace to the Hub.
			</p>
			{#if loadingSchema}
				<div class="flex items-center gap-2 text-secondary text-sm">
					<Loader2 class="animate-spin" size={16} /> Loading inputs…
				</div>
			{:else if recordSchema}
				<SchemaForm bind:args={recordArgs} bind:isValid={recordValid} schema={recordSchema} />
			{:else}
				<p class="text-sm text-secondary">This item has no inputs.</p>
			{/if}
		</div>
		{#snippet actions()}
			<Button variant="default" onclick={cancelRecord}>Cancel</Button>
			<Button
				variant="accent"
				disabled={loadingSchema || !recordValid}
				startIcon={{ icon: Play }}
				onclick={confirmRecord}
			>
				Run & record
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

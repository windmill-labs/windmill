<script lang="ts">
	import { ScriptService, FlowService, WorkspaceService, type FlowModule } from '$lib/gen'
	import { Check, X, RefreshCcw, ChevronDown, ChevronRight, CodeXml } from 'lucide-svelte'
	import { Button } from './common'
	import Select from './select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import Badge from './common/badge/Badge.svelte'
	import { SvelteMap } from 'svelte/reactivity'
	import { untrack } from 'svelte'
	import BarsStaggered from './icons/BarsStaggered.svelte'
	import { parseTag } from './dedicated_worker'

	// A "Runnable" is a script or flow with dedicated_worker=true
	interface Runnable {
		tag: string // workspace:path or workspace:flow/path
		displayName: string
		language: string
		type: 'script' | 'flow'
		path: string
		selected: boolean
		// For flows, the actual runners (steps) that will be spawned
		runners?: FlowRunner[]
		loadingRunners?: boolean
		expanded?: boolean
	}

	// A "FlowRunner" is an individual step within a flow that will get a dedicated worker
	interface FlowRunner {
		stepId: string
		stepSummary?: string
		language?: string
		scriptPath?: string
		isInline: boolean
	}

	interface Props {
		selectedTags: string[]
		disabled?: boolean
		onchange?: (tags: string[]) => void
	}

	let { selectedTags = $bindable([]), disabled = false, onchange }: Props = $props()

	let selectedWorkspace: string | undefined = $state(undefined)
	let runnables: Runnable[] = $state([])
	let loading = $state(false)
	let workspaces: { id: string; name: string }[] = $state([])
	let workspacesLoading = $state(true)
	let selectorExpanded = $state(false)

	// Track detailed info for each selected tag (for displaying in summary)
	interface SelectedTagInfo {
		tag: string
		workspace: string
		type: 'script' | 'flow'
		path: string
		runners?: FlowRunner[]
		expanded?: boolean
		loading?: boolean
	}
	let selectedTagsInfo: SvelteMap<string, SelectedTagInfo> = $state(new SvelteMap())

	// Languages that support dedicated workers
	const DEDICATED_WORKER_LANGUAGES = ['python3', 'bun', 'bunnative', 'deno']

	// Resolve workspace script languages and filter to supported languages
	async function resolveAndFilterRunners(
		workspace: string,
		preliminaryRunners: FlowRunner[]
	): Promise<FlowRunner[]> {
		const runnersWithLanguage = await Promise.all(
			preliminaryRunners.map(async (runner) => {
				if (!runner.isInline && runner.scriptPath) {
					try {
						const script = await ScriptService.getScriptByPath({
							workspace,
							path: runner.scriptPath
						})
						return { ...runner, language: script.language }
					} catch (e) {
						console.error(`Failed to fetch script ${runner.scriptPath}`, e)
						return { ...runner, language: undefined }
					}
				}
				return runner
			})
		)

		// Filter to only supported languages
		return runnersWithLanguage.filter(
			(runner) => runner.language && DEDICATED_WORKER_LANGUAGES.includes(runner.language)
		)
	}

	// Load detailed info for all selected tags
	async function loadSelectedTagsInfo(tags: string[]) {
		if (tags.length === 0) {
			selectedTagsInfo = new SvelteMap()
			return
		}

		// Capture current state without tracking to avoid infinite loops
		const currentInfo = untrack(() => selectedTagsInfo)
		const currentRunnables = untrack(() => runnables)

		const newInfo = new SvelteMap<string, SelectedTagInfo>()

		try {
			await Promise.all(
				tags.map(async (tag) => {
					// Check if we already have this info cached
					const existing = currentInfo.get(tag)
					if (existing && (existing.type === 'script' || existing.runners !== undefined)) {
						newInfo.set(tag, existing)
						return
					}

					// Check if we have it loaded in runnables
					const existingRunnable = currentRunnables.find((r) => r.tag === tag)
					if (existingRunnable) {
						newInfo.set(tag, {
							tag,
							workspace: tag.substring(0, tag.indexOf(':')),
							type: existingRunnable.type,
							path: existingRunnable.path,
							runners: existingRunnable.runners,
							expanded: existing?.expanded ?? false
						})
						return
					}

					// Parse and fetch
					const parsed = parseTag(tag)
					if (!parsed) return

					if (parsed.type === 'script') {
						newInfo.set(tag, {
							tag,
							workspace: parsed.workspace,
							type: 'script',
							path: parsed.path
						})
					} else {
						// Flows need to fetch to get runners
						try {
							const flow = await FlowService.getFlowByPath({
								workspace: parsed.workspace,
								path: parsed.path
							})
							const preliminaryRunners = flow.value?.modules
								? extractRunnersFromModules(flow.value.modules)
								: []
							const runners = await resolveAndFilterRunners(parsed.workspace, preliminaryRunners)
							newInfo.set(tag, {
								tag,
								workspace: parsed.workspace,
								type: 'flow',
								path: parsed.path,
								runners,
								expanded: existing?.expanded ?? false
							})
						} catch (e) {
							console.error(`Failed to load flow ${parsed.path}`, e)
							newInfo.set(tag, {
								tag,
								workspace: parsed.workspace,
								type: 'flow',
								path: parsed.path,
								runners: []
							})
						}
					}
				})
			)
		} finally {
			selectedTagsInfo = newInfo
		}
	}

	function toggleSelectedTagExpanded(tag: string) {
		const info = selectedTagsInfo.get(tag)
		if (info) {
			// Need to set the whole object to trigger reactivity
			selectedTagsInfo.set(tag, { ...info, expanded: !info.expanded })
		}
	}

	// Auto-expand selector if no tags selected
	$effect(() => {
		if (selectedTags.length === 0) {
			selectorExpanded = true
		}
	})

	// Load selected tags info when selectedTags change
	$effect(() => {
		loadSelectedTagsInfo(selectedTags)
	})

	$effect(() => {
		loadWorkspaces()
	})

	async function loadWorkspaces() {
		try {
			workspacesLoading = true
			const ws = await WorkspaceService.listWorkspaces()
			workspaces = ws.map((w) => ({ id: w.id, name: w.name }))
		} catch (e) {
			console.error('Failed to load workspaces', e)
			sendUserToast('Failed to load workspaces', true)
		} finally {
			workspacesLoading = false
		}
	}

	// Extract runners from flow modules recursively
	// Returns runners with language info for inline scripts, and scriptPath for workspace scripts
	function extractRunnersFromModules(modules: FlowModule[]): FlowRunner[] {
		const runners: FlowRunner[] = []

		for (const module of modules) {
			const value = module.value
			switch (value.type) {
				case 'rawscript':
					if (DEDICATED_WORKER_LANGUAGES.includes(value.language)) {
						runners.push({
							stepId: module.id,
							stepSummary: module.summary,
							language: value.language,
							scriptPath: value.path,
							isInline: true
						})
					}
					break
				case 'script':
					// For workspace script references, we'll resolve the language later
					runners.push({
						stepId: module.id,
						stepSummary: module.summary,
						language: undefined, // Will be resolved by fetching the script
						scriptPath: value.path,
						isInline: false
					})
					break
				case 'forloopflow':
					runners.push(...extractRunnersFromModules(value.modules))
					break
				case 'whileloopflow':
					runners.push(...extractRunnersFromModules(value.modules))
					break
				case 'branchone':
					for (const branch of value.branches) {
						runners.push(...extractRunnersFromModules(branch.modules))
					}
					runners.push(...extractRunnersFromModules(value.default))
					break
				case 'branchall':
					for (const branch of value.branches) {
						runners.push(...extractRunnersFromModules(branch.modules))
					}
					break
			}
		}

		return runners
	}

	async function loadFlowRunners(runnable: Runnable) {
		if (!selectedWorkspace || runnable.type !== 'flow') return

		try {
			runnable.loadingRunners = true
			const flow = await FlowService.getFlowByPath({
				workspace: selectedWorkspace,
				path: runnable.path
			})

			if (flow.value?.modules) {
				const preliminaryRunners = extractRunnersFromModules(flow.value.modules)
				runnable.runners = await resolveAndFilterRunners(selectedWorkspace, preliminaryRunners)
			} else {
				runnable.runners = []
			}
		} catch (e) {
			console.error('Failed to load flow runners', e)
			runnable.runners = []
		} finally {
			runnable.loadingRunners = false
		}
	}

	async function loadRunnables(workspaceId: string) {
		try {
			loading = true
			runnables = []

			const [scripts, flows] = await Promise.all([
				ScriptService.listScripts({
					workspace: workspaceId,
					dedicatedWorker: true
				}),
				FlowService.listFlows({
					workspace: workspaceId,
					dedicatedWorker: true
				})
			])

			const newRunnables: Runnable[] = []

			// Add scripts with supported languages
			for (const script of scripts) {
				if (DEDICATED_WORKER_LANGUAGES.includes(script.language ?? '')) {
					const tag = `${workspaceId}:${script.path}`
					newRunnables.push({
						tag,
						displayName: script.path,
						language: script.language ?? 'unknown',
						type: 'script',
						path: script.path,
						selected: selectedTags.includes(tag)
					})
				}
			}

			// Add flows
			for (const flow of flows) {
				const tag = `${workspaceId}:flow/${flow.path}`
				newRunnables.push({
					tag,
					displayName: flow.path,
					language: 'flow',
					type: 'flow',
					path: flow.path,
					selected: selectedTags.includes(tag),
					runners: undefined,
					loadingRunners: false,
					expanded: false
				})
			}

			runnables = newRunnables

			// Load runners for all flows in parallel
			await Promise.all(runnables.filter((r) => r.type === 'flow').map((r) => loadFlowRunners(r)))
		} catch (e) {
			console.error('Failed to load runnables', e)
			sendUserToast('Failed to load scripts/flows', true)
		} finally {
			loading = false
		}
	}

	function toggleRunnable(runnable: Runnable) {
		runnable.selected = !runnable.selected
		updateSelectedTags()
	}

	function toggleExpanded(runnable: Runnable) {
		runnable.expanded = !runnable.expanded
	}

	function selectAll() {
		for (const runnable of runnables) {
			runnable.selected = true
		}
		updateSelectedTags()
	}

	function deselectAll() {
		for (const runnable of runnables) {
			runnable.selected = false
		}
		updateSelectedTags()
	}

	function updateSelectedTags() {
		selectedTags = runnables.filter((r) => r.selected).map((r) => r.tag)
		onchange?.(selectedTags)
	}

	function removeTag(tag: string) {
		selectedTags = selectedTags.filter((t) => t !== tag)
		// Also update runnable state if visible
		const runnable = runnables.find((r) => r.tag === tag)
		if (runnable) {
			runnable.selected = false
		}
		onchange?.(selectedTags)
	}

	$effect(() => {
		if (selectedWorkspace) {
			loadRunnables(selectedWorkspace)
		}
	})

	let selectedCount = $derived(runnables.filter((r) => r.selected).length)
</script>

<div class="flex flex-col gap-3">
	<!-- Selected tags summary -->
	{#if selectedTags.length > 0}
		<div class="flex flex-col gap-2">
			<div class="border rounded-md divide-y bg-surface max-h-48 overflow-y-auto">
				{#each selectedTags as tag (tag)}
					{@const info = selectedTagsInfo.get(tag)}
					<div>
						<div class="flex items-center">
							{#if info?.type === 'flow' && info.runners && info.runners.length > 0}
								<button
									class="p-2 hover:bg-surface-hover transition-colors"
									onclick={(e) => {
										e.stopPropagation()
										toggleSelectedTagExpanded(tag)
									}}
								>
									{#if info.expanded}
										<ChevronDown class="h-3 w-3 text-tertiary" />
									{:else}
										<ChevronRight class="h-3 w-3 text-tertiary" />
									{/if}
								</button>
							{:else}
								<div class="w-7"></div>
							{/if}
							<div class="flex-1 flex flex-col gap-0.5 px-2 py-1.5 min-w-0">
								<div class="flex items-center gap-2 min-w-0">
									{#if info}
										{#if info.type === 'flow'}
											<BarsStaggered size={14} class="flex-shrink-0 text-secondary" />
										{:else}
											<CodeXml size={14} class="flex-shrink-0 text-secondary" />
										{/if}
										<span class="text-xs truncate flex-1 min-w-0">{info.path}</span>
										<span class="text-xs text-tertiary flex-shrink-0">({info.workspace})</span>
										{#if info.type === 'flow' && info.runners}
											<Badge color="indigo" small>
												{info.runners.length} runner{info.runners.length !== 1 ? 's' : ''}
											</Badge>
										{:else if info.type === 'script'}
											<Badge color="blue" small>1 runner</Badge>
										{/if}
									{:else}
										<span class="text-xs text-tertiary truncate">{tag}</span>
									{/if}
								</div>
							</div>
							{#if !disabled}
								<button
									class="p-2 hover:text-red-500 transition-colors"
									onclick={(e) => {
										e.stopPropagation()
										removeTag(tag)
									}}
								>
									<X class="h-3 w-3" />
								</button>
							{/if}
						</div>

						{#if info?.type === 'flow' && info.expanded && info.runners}
							<div class="bg-surface-secondary border-t">
								{#each info.runners as runner (runner.stepId)}
									<div
										class="flex items-center gap-2 px-9 py-1 text-xs border-t first:border-t-0 min-w-0"
									>
										<span class="font-mono text-tertiary flex-shrink-0">{runner.stepId}</span>
										{#if runner.stepSummary}
											<span class="text-secondary truncate flex-1 min-w-0"
												>{runner.stepSummary}</span
											>
										{/if}
										<Badge color="gray" small>
											{runner.isInline ? runner.language : runner.scriptPath}
										</Badge>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Collapsible selector section -->
	<div class="border rounded-md">
		<button
			class="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-surface-hover transition-colors"
			onclick={() => (selectorExpanded = !selectorExpanded)}
			{disabled}
		>
			{#if selectorExpanded}
				<ChevronDown class="h-4 w-4 text-secondary" />
			{:else}
				<ChevronRight class="h-4 w-4 text-secondary" />
			{/if}
			<span class="text-sm">
				{selectedTags.length > 0 ? 'Add more scripts/flows' : 'Select scripts/flows'}
			</span>
		</button>

		{#if selectorExpanded}
			<div class="border-t px-3 py-3 flex flex-col gap-3">
				<!-- Workspace selector -->
				<div class="flex flex-col gap-1">
					<span class="text-xs text-secondary">Workspace</span>
					<Select
						bind:value={selectedWorkspace}
						items={workspaces.map((w) => ({ value: w.id, label: `${w.name} (${w.id})` }))}
						placeholder="Select workspace..."
						disabled={disabled || workspacesLoading}
					/>
				</div>

				<!-- Scripts/flows list -->
				{#if selectedWorkspace}
					<div class="flex flex-col gap-2">
						<div class="flex items-center justify-between">
							<span class="text-xs text-secondary">Scripts/flows with dedicated worker enabled</span
							>
							{#if !loading && runnables.length > 0}
								<div class="flex gap-1">
									<Button size="xs2" color="light" on:click={selectAll} {disabled}>All</Button>
									<Button size="xs2" color="light" on:click={deselectAll} {disabled}>None</Button>
									<Button
										size="xs2"
										color="light"
										iconOnly
										startIcon={{ icon: RefreshCcw }}
										on:click={() => selectedWorkspace && loadRunnables(selectedWorkspace)}
										{disabled}
									/>
								</div>
							{/if}
						</div>

						{#if loading}
							<div class="flex items-center justify-center py-4">
								<RefreshCcw class="animate-spin h-4 w-4 text-secondary" />
								<span class="ml-2 text-xs text-secondary">Loading...</span>
							</div>
						{:else if runnables.length === 0}
							<div class="text-xs text-tertiary py-3 text-center">
								No scripts or flows with dedicated worker enabled found.
							</div>
						{:else}
							<div class="border rounded-md divide-y max-h-64 overflow-y-auto bg-surface">
								{#each runnables as runnable (runnable.tag)}
									<div>
										<div class="flex items-center">
											{#if runnable.type === 'flow' && runnable.runners && runnable.runners.length > 0}
												<button
													class="p-2 hover:bg-surface-hover transition-colors"
													onclick={(e) => {
														e.stopPropagation()
														toggleExpanded(runnable)
													}}
													{disabled}
												>
													{#if runnable.expanded}
														<ChevronDown class="h-3 w-3 text-tertiary" />
													{:else}
														<ChevronRight class="h-3 w-3 text-tertiary" />
													{/if}
												</button>
											{:else}
												<div class="w-7"></div>
											{/if}
											<button
												class="flex-1 flex items-center gap-2 px-2 py-1.5 hover:bg-surface-hover transition-colors text-left min-w-0"
												onclick={(e) => {
													e.stopPropagation()
													if (!disabled) toggleRunnable(runnable)
												}}
												{disabled}
											>
												<div
													class="w-4 h-4 border rounded flex items-center justify-center flex-shrink-0"
													class:bg-blue-500={runnable.selected}
													class:border-blue-500={runnable.selected}
												>
													{#if runnable.selected}
														<Check class="h-3 w-3 text-white" />
													{/if}
												</div>
												<span class="flex-1 text-xs truncate min-w-0"
													>{runnable.displayName}</span
												>
												{#if runnable.type === 'flow' && runnable.runners}
													<span class="text-xs text-tertiary flex-shrink-0">
														{runnable.runners.length}
													</span>
												{/if}
												<Badge color={runnable.type === 'flow' ? 'indigo' : 'blue'} small>
													{runnable.type === 'flow' ? 'flow' : runnable.language}
												</Badge>
											</button>
										</div>

										{#if runnable.type === 'flow' && runnable.expanded && runnable.runners}
											<div class="bg-surface-secondary border-t">
												{#if runnable.runners.length === 0}
													<div class="px-9 py-1.5 text-xs text-tertiary italic">
														No eligible steps (python3/bun/bunnative/deno)
													</div>
												{:else}
													{#each runnable.runners as runner (runner.stepId)}
														<div
															class="flex items-center gap-2 px-9 py-1 text-xs border-t first:border-t-0 min-w-0"
														>
															<span class="font-mono text-tertiary flex-shrink-0"
																>{runner.stepId}</span
															>
															{#if runner.stepSummary}
																<span class="text-secondary truncate flex-1 min-w-0">
																	{runner.stepSummary}
																</span>
															{/if}
															<Badge color="gray" small>
																{runner.isInline ? runner.language : runner.scriptPath}
															</Badge>
														</div>
													{/each}
												{/if}
											</div>
										{/if}
									</div>
								{/each}
							</div>
							<div class="text-xs text-tertiary">
								{selectedCount} selected
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

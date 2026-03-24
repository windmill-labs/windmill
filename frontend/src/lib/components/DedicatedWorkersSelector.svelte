<script lang="ts">
	import {
		ScriptService,
		FlowService,
		WorkspaceService,
		WorkspaceDependenciesService,
		type FlowModule
	} from '$lib/gen'
	import {
		Check,
		X,
		RefreshCcw,
		ChevronDown,
		ChevronRight,
		CodeXml,
		ExternalLink,
		TriangleAlert,
		Layers
	} from 'lucide-svelte'
	import { Button } from './common'
	import Select from './select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import Badge from './common/badge/Badge.svelte'
	import { SvelteMap } from 'svelte/reactivity'
	import { untrack } from 'svelte'
	import BarsStaggered from './icons/BarsStaggered.svelte'
	import { parseTag } from './dedicated_worker'
	import Tooltip from './Tooltip.svelte'

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
		// Workspace dependency names from annotations (scripts only)
		workspaceDeps?: string[]
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
	// Set of existing workspace dependency names for the current workspace (for validation)
	let existingDeps: Set<string> = $state(new Set())

	// Track detailed info for each selected tag (for displaying in summary)
	interface SelectedTagInfo {
		tag: string
		workspace: string
		type: 'script' | 'flow'
		path: string
		language?: string
		runners?: FlowRunner[]
		expanded?: boolean
		loading?: boolean
		workspaceDeps?: string[]
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
		const currentExistingDeps = untrack(() => existingDeps)

		const newInfo = new SvelteMap<string, SelectedTagInfo>()

		// Collect workspaces that need dep info fetched
		const workspacesNeedingDeps = new Set<string>()
		for (const tag of tags) {
			const existing = currentInfo.get(tag)
			const existingRunnable = currentRunnables.find((r) => r.tag === tag)
			// If we don't have workspaceDeps cached, we need to fetch for this workspace
			if (!existing?.workspaceDeps && !existingRunnable?.workspaceDeps) {
				const parsed = parseTag(tag)
				if (parsed?.type === 'script') {
					workspacesNeedingDeps.add(parsed.workspace)
				}
			}
		}

		// Fetch workspace dep info for workspaces that need it
		const depsPerWorkspace = new Map<string, Map<string, { deps: string[]; language: string }>>()
		if (workspacesNeedingDeps.size > 0 || currentExistingDeps.size === 0) {
			await Promise.all(
				Array.from(workspacesNeedingDeps).map(async (ws) => {
					try {
						const [dedicatedDeps, wsDeps] = await Promise.all([
							ScriptService.listDedicatedWithDeps({ workspace: ws }).catch(() => []),
							WorkspaceDependenciesService.listWorkspaceDependencies({
								workspace: ws
							}).catch(() => [])
						])
						const depsMap = new Map<string, { deps: string[]; language: string }>()
						for (const d of dedicatedDeps) {
							if (d.workspace_dep_names.length > 0) {
								depsMap.set(d.path, {
									deps: d.workspace_dep_names,
									language: d.language
								})
							}
						}
						depsPerWorkspace.set(ws, depsMap)
						// Merge into existingDeps
						for (const d of wsDeps) {
							if (!d.archived && d.name) {
								currentExistingDeps.add(d.name)
							}
						}
					} catch {
						// ignore
					}
				})
			)
			existingDeps = new Set(currentExistingDeps)
		}

		try {
			await Promise.all(
				tags.map(async (tag) => {
					// Check if we already have this info cached
					const existing = currentInfo.get(tag)
					if (existing && (existing.type === 'script' || existing.runners !== undefined)) {
						// Backfill workspaceDeps and language if missing
						if (existing.type === 'script' && !existing.workspaceDeps) {
							const depInfo = depsPerWorkspace.get(existing.workspace)?.get(existing.path)
							if (depInfo) {
								existing.workspaceDeps = depInfo.deps
								existing.language = depInfo.language
							}
						}
						newInfo.set(tag, existing)
						return
					}

					// Check if we have it loaded in runnables
					const existingRunnable = currentRunnables.find((r) => r.tag === tag)
					if (existingRunnable) {
						const ws = tag.substring(0, tag.indexOf(':'))
						const depInfo = depsPerWorkspace.get(ws)?.get(existingRunnable.path)
						newInfo.set(tag, {
							tag,
							workspace: ws,
							type: existingRunnable.type,
							path: existingRunnable.path,
							language: existingRunnable.language,
							runners: existingRunnable.runners,
							expanded: existing?.expanded ?? false,
							workspaceDeps: existingRunnable.workspaceDeps ?? depInfo?.deps
						})
						return
					}

					// Parse and fetch
					const parsed = parseTag(tag)
					if (!parsed) return

					if (parsed.type === 'script') {
						const depInfo = depsPerWorkspace.get(parsed.workspace)?.get(parsed.path)
						newInfo.set(tag, {
							tag,
							workspace: parsed.workspace,
							type: 'script',
							path: parsed.path,
							language: depInfo?.language,
							workspaceDeps: depInfo?.deps
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

			const [scripts, flows, dedicatedDeps, wsDeps] = await Promise.all([
				ScriptService.listScripts({
					workspace: workspaceId,
					dedicatedWorker: true
				}),
				FlowService.listFlows({
					workspace: workspaceId,
					dedicatedWorker: true
				}),
				ScriptService.listDedicatedWithDeps({
					workspace: workspaceId
				}).catch(() => []),
				WorkspaceDependenciesService.listWorkspaceDependencies({
					workspace: workspaceId
				}).catch(() => [])
			])

			// Track existing workspace dep names for validation
			existingDeps = new Set(
				wsDeps
					.filter((d) => !d.archived)
					.map((d) => d.name)
					.filter((n): n is string => !!n)
			)

			// Build a map from path -> workspace dep names
			const depsMap = new Map<string, string[]>()
			for (const d of dedicatedDeps) {
				if (d.workspace_dep_names.length > 0) {
					depsMap.set(d.path, d.workspace_dep_names)
				}
			}

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
						selected: selectedTags.includes(tag),
						workspaceDeps: depsMap.get(script.path)
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

	// Compute shared runner groups: scripts sharing a (dep_name, language) pair
	interface RunnerGroup {
		depName: string
		language: string
		tags: string[]
	}

	let runnerGroups: RunnerGroup[] = $derived.by(() => {
		const groupMap = new Map<string, { depName: string; language: string; tags: string[] }>()
		for (const tag of selectedTags) {
			const info = selectedTagsInfo.get(tag)
			if (info?.type === 'script' && info.workspaceDeps) {
				const lang = info.language ?? runnables.find((r) => r.tag === tag)?.language ?? 'unknown'
				for (const dep of info.workspaceDeps) {
					const key = `${dep}:${lang}`
					const existing = groupMap.get(key)
					if (existing) {
						existing.tags.push(tag)
					} else {
						groupMap.set(key, { depName: dep, language: lang, tags: [tag] })
					}
				}
			}
		}
		// Only return groups with 2+ scripts
		return Array.from(groupMap.values()).filter((g) => g.tags.length >= 2)
	})

	// Map tag → runner group it belongs to
	let tagRunnerGroup: Map<string, RunnerGroup> = $derived.by(() => {
		const map = new Map<string, RunnerGroup>()
		for (const group of runnerGroups) {
			for (const tag of group.tags) {
				map.set(tag, group)
			}
		}
		return map
	})

	// Tags not in any runner group (standalone dedicated workers)
	let standaloneTags: string[] = $derived(selectedTags.filter((tag) => !tagRunnerGroup.has(tag)))
</script>

{#snippet tagRow(tag: string, info: SelectedTagInfo | undefined)}
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
			<div class="flex-1 flex items-center gap-2 px-2 py-1.5 min-w-0">
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
					{/if}
					{#if info.workspaceDeps && !tagRunnerGroup.has(tag)}
						{#each info.workspaceDeps as dep}
							{#if existingDeps.has(dep)}
								<Badge color="indigo" small href="/workspace_settings?tab=dependencies">
									{dep}
									<ExternalLink class="h-2.5 w-2.5" />
								</Badge>
							{:else}
								<Tooltip small>
									Workspace dependency '{dep}' not found. Create it in workspace settings to enable
									shared runners.
								</Tooltip>
								<Badge color="yellow" small>
									<TriangleAlert class="h-2.5 w-2.5" />
									{dep}
								</Badge>
							{/if}
						{/each}
					{/if}
				{:else}
					<span class="text-xs text-tertiary truncate">{tag}</span>
				{/if}
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
					<div class="flex items-center gap-2 px-9 py-1 text-xs border-t first:border-t-0 min-w-0">
						<span class="font-mono text-tertiary flex-shrink-0">{runner.stepId}</span>
						{#if runner.stepSummary}
							<span class="text-secondary truncate flex-1 min-w-0">{runner.stepSummary}</span>
						{/if}
						<Badge color="gray" small>
							{runner.isInline ? runner.language : runner.scriptPath}
						</Badge>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<div class="flex flex-col gap-3">
	{#if selectedTags.length > 0}
		<div class="flex flex-col gap-2">
			<div class="border rounded-md bg-surface max-h-64 overflow-y-auto divide-y">
				<!-- Shared runner groups -->
				{#each runnerGroups as group (`${group.depName}:${group.language}`)}
					<div>
						<div class="flex items-center gap-2 px-3 py-1.5 bg-surface-secondary border-b">
							<Layers size={12} class="flex-shrink-0 text-secondary" />
							<span class="text-xs font-medium text-emphasis">Shared runner</span>
							<span class="text-xs text-tertiary">·</span>
							<span class="text-xs text-tertiary">{group.language}</span>
							<span class="flex-1"></span>
							{#if existingDeps.has(group.depName)}
								<Badge color="indigo" small href="/workspace_settings?tab=dependencies">
									{group.depName}
									<ExternalLink class="h-2.5 w-2.5" />
								</Badge>
							{:else}
								<Tooltip small>
									Workspace dependency '{group.depName}' not found. Create it in workspace settings
									for shared runner to work.
								</Tooltip>
								<Badge color="yellow" small>
									<TriangleAlert class="h-2.5 w-2.5" />
									{group.depName}
								</Badge>
							{/if}
						</div>
						<div class="divide-y">
							{#each group.tags as tag (tag)}
								{@const info = selectedTagsInfo.get(tag)}
								{@render tagRow(tag, info)}
							{/each}
						</div>
					</div>
				{/each}

				<!-- Standalone scripts/flows (not in any runner group) -->
				{#each standaloneTags as tag (tag)}
					{@const info = selectedTagsInfo.get(tag)}
					{@render tagRow(tag, info)}
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
												<span class="flex-1 text-xs truncate min-w-0">{runnable.displayName}</span>
												{#if runnable.type === 'flow' && runnable.runners}
													<span class="text-xs text-tertiary flex-shrink-0">
														{runnable.runners.length}
													</span>
												{/if}
												{#if runnable.workspaceDeps}
													{#each runnable.workspaceDeps as dep}
														{#if existingDeps.has(dep)}
															<Badge color="indigo" small>{dep}</Badge>
														{:else}
															<Tooltip small>
																Workspace dependency '{dep}' not found. Create it in workspace
																settings to enable shared runners.
															</Tooltip>
															<Badge color="yellow" small>
																<TriangleAlert class="h-2.5 w-2.5" />
																{dep}
															</Badge>
														{/if}
													{/each}
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

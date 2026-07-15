<script lang="ts">
	import { Alert, Badge, Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { FlowService, FolderService, IntegrationService, ScriptService } from '$lib/gen'
	import { mcpEndpointTools } from '$lib/mcpEndpointTools'
	import InfoIcon from 'lucide-svelte/icons/info'
	import { SvelteMap } from 'svelte/reactivity'

	interface Props {
		workspaceId: string
		scope: string
		initialScope?: string
		readOnly?: boolean
	}

	let { workspaceId, scope = $bindable(), initialScope, readOnly = false }: Props = $props()

	// Mirrors ITEMS_FETCH_MAX_LIMIT in backend/windmill-api/src/mcp/utils.rs: the
	// server exposes at most this many scripts and this many flows (per type) as
	// MCP tools, taking the most recently created. A scope matching more than this
	// silently drops the overflow, so we warn at configuration time.
	const MCP_TOOL_FETCH_LIMIT = 100

	// Endpoints we can actually advertise to a read-only MCP token. Mirrors the
	// runner's filter (only GET endpoints).
	const visibleEndpointTools = $derived(
		readOnly ? mcpEndpointTools.filter((e) => e.method === 'GET') : mcpEndpointTools
	)

	// When read-only flips on, prune already-selected non-GET endpoints so the
	// scope string doesn't keep references to tools the server will reject.
	$effect(() => {
		if (!readOnly || selectedEndpoints.length === 0) return
		const allowed = new Set(visibleEndpointTools.map((e) => e.name))
		const filtered = selectedEndpoints.filter((n) => allowed.has(n))
		if (filtered.length !== selectedEndpoints.length) {
			selectedEndpoints = filtered
		}
	})

	const parsedInitial = parseInitialScope(initialScope)

	let selectedMode = $state<'favorites' | 'all' | 'folder' | 'custom'>(parsedInitial.mode)
	let selectedFolders = $state<string[]>(parsedInitial.folders)
	let allFolders = $state<string[]>([])
	let loadingFolders = $state(false)
	let folderNamesCache = new Map<string, string[]>()
	let selectedScripts = $state<string[]>(parsedInitial.scripts)
	let selectedFlows = $state<string[]>(parsedInitial.flows)
	let selectedEndpoints = $state<string[]>(parsedInitial.endpoints)
	let customScriptPatterns = $state<string>(parsedInitial.scriptPatterns)
	let customFlowPatterns = $state<string>(parsedInitial.flowPatterns)
	let newMcpApps = $state<string[]>(parsedInitial.hubApps)

	let allScripts = $state<string[]>([])
	let allFlows = $state<string[]>([])
	let allApps = $state<string[]>([])
	let loadingApps = $state(false)
	let errorFetchApps = $state(false)
	let loadingRunnables = $state(false)
	let includedRunnables = $state<string[]>([])

	let runnablesCache = new SvelteMap<string, { scripts: string[]; flows: string[] }>()

	function parsePatterns(input: string): string[] {
		return input
			.split(',')
			.map((p) => p.trim())
			.filter((p) => p.length > 0)
	}

	type ParsedScope = {
		mode: 'favorites' | 'all' | 'folder' | 'custom'
		folders: string[]
		scripts: string[]
		flows: string[]
		endpoints: string[]
		scriptPatterns: string
		flowPatterns: string
		hubApps: string[]
	}

	function parseInitialScope(input: string | undefined): ParsedScope {
		const empty: ParsedScope = {
			mode: 'favorites',
			folders: [],
			scripts: [],
			flows: [],
			endpoints: [],
			scriptPatterns: '',
			flowPatterns: '',
			hubApps: []
		}
		if (!input) return empty

		const parts = input.split(/\s+/).filter((p) => p.length > 0)
		if (parts.length === 0) return empty

		const byKind: Record<string, string[]> = {}
		let mode: ParsedScope['mode'] = 'custom'
		const hubApps: string[] = []

		for (const part of parts) {
			if (part === 'mcp:favorites') {
				mode = 'favorites'
			} else if (part === 'mcp:all') {
				mode = 'all'
			} else if (part.startsWith('mcp:hub:')) {
				hubApps.push(...parsePatterns(part.slice('mcp:hub:'.length)))
			} else if (part.startsWith('mcp:scripts:')) {
				byKind.scripts = parsePatterns(part.slice('mcp:scripts:'.length))
			} else if (part.startsWith('mcp:flows:')) {
				byKind.flows = parsePatterns(part.slice('mcp:flows:'.length))
			} else if (part.startsWith('mcp:endpoints:')) {
				byKind.endpoints = parsePatterns(part.slice('mcp:endpoints:'.length))
			}
		}

		// Detect folder mode: scripts and flows are exclusively `f/X/*` patterns
		// for the same set of folders, and endpoints is exactly `*`.
		const folderRe = /^f\/([^/]+)\/\*$/
		const scriptFolders = (byKind.scripts ?? []).map((p) => p.match(folderRe)?.[1])
		const flowFolders = (byKind.flows ?? []).map((p) => p.match(folderRe)?.[1])
		const allScriptsAreFolders = scriptFolders.length > 0 && scriptFolders.every((f) => !!f)
		const allFlowsAreFolders = flowFolders.length > 0 && flowFolders.every((f) => !!f)
		const sameFolders =
			allScriptsAreFolders &&
			allFlowsAreFolders &&
			scriptFolders.length === flowFolders.length &&
			scriptFolders.every((f, i) => f === flowFolders[i])
		const endpointsIsAll = byKind.endpoints?.length === 1 && byKind.endpoints[0] === '*'

		if (mode !== 'favorites' && mode !== 'all' && sameFolders && endpointsIsAll) {
			return {
				mode: 'folder',
				folders: scriptFolders.filter((f): f is string => !!f),
				scripts: [],
				flows: [],
				endpoints: [],
				scriptPatterns: '',
				flowPatterns: '',
				hubApps
			}
		}

		if (mode === 'favorites' || mode === 'all') {
			return { ...empty, mode, hubApps }
		}

		// Custom mode: split each list into "selectable" entries (later filtered
		// against allScripts/allFlows once loaded) and free-form patterns.
		// We can't know yet which are real paths vs wildcard patterns, so pass
		// everything as patterns; once allScripts/allFlows load, $effect will
		// move matching entries into selectedScripts/selectedFlows.
		return {
			mode: 'custom',
			folders: [],
			scripts: [],
			flows: [],
			endpoints: byKind.endpoints ?? [],
			scriptPatterns: (byKind.scripts ?? []).join(','),
			flowPatterns: (byKind.flows ?? []).join(','),
			hubApps
		}
	}

	// Compute scope string from selections
	$effect(() => {
		let scopeParts: string[] = []

		if (selectedMode === 'custom') {
			let scriptPaths = [...selectedScripts]
			if (customScriptPatterns.trim()) {
				scriptPaths.push(...parsePatterns(customScriptPatterns))
			}
			if (scriptPaths.length > 0) {
				scopeParts.push(`mcp:scripts:${scriptPaths.join(',')}`)
			}

			let flowPaths = [...selectedFlows]
			if (customFlowPatterns.trim()) {
				flowPaths.push(...parsePatterns(customFlowPatterns))
			}
			if (flowPaths.length > 0) {
				scopeParts.push(`mcp:flows:${flowPaths.join(',')}`)
			}

			if (selectedEndpoints.length > 0) {
				scopeParts.push(`mcp:endpoints:${selectedEndpoints.join(',')}`)
			}
		} else if (selectedMode === 'folder') {
			const folderPaths = selectedFolders.map((f) => `f/${f}/*`).join(',')
			if (selectedFolders.length > 0) {
				scopeParts = [`mcp:scripts:${folderPaths}`, `mcp:flows:${folderPaths}`, `mcp:endpoints:*`]
			}
		} else {
			scopeParts = [`mcp:${selectedMode}`]
		}

		if (newMcpApps.length > 0) {
			scopeParts.push(`mcp:hub:${newMcpApps.join(',')}`)
		}

		scope = scopeParts.join(' ')
	})

	// Clear pattern inputs when not in custom scope
	$effect(() => {
		if (selectedMode !== 'custom') {
			customScriptPatterns = ''
			customFlowPatterns = ''
		}
	})

	// Clear folders when not in folder mode, load folder names when entering folder mode
	$effect(() => {
		if (selectedMode === 'folder' && workspaceId) {
			loadFolderNames(workspaceId)
		} else {
			selectedFolders = []
		}
	})

	async function loadFolderNames(workspace: string) {
		if (folderNamesCache.has(workspace)) {
			allFolders = folderNamesCache.get(workspace)!
			return
		}
		try {
			loadingFolders = true
			const excludedFolders = ['app_groups', 'app_custom', 'app_themes']
			const names = (await FolderService.listFolderNames({ workspace })).filter(
				(x) => !excludedFolders.includes(x)
			)
			folderNamesCache.set(workspace, names)
			allFolders = names
		} catch {
			allFolders = []
		} finally {
			loadingFolders = false
		}
	}

	// Load hub apps on mount
	async function getAllApps() {
		if (allApps.length > 0) return
		try {
			loadingApps = true
			allApps = (
				await IntegrationService.listHubIntegrations({
					kind: 'script'
				})
			).map((x) => x.name)
		} catch (err) {
			console.error('Hub is not available')
			allApps = []
			errorFetchApps = true
		} finally {
			loadingApps = false
		}
	}

	getAllApps()

	async function getScripts(
		favoriteOnly: boolean = false,
		workspace: string,
		folder: string | undefined
	) {
		if (!workspace) return []
		const pathStart = folder ? `f/${folder}` : undefined
		const scripts = await ScriptService.listScripts({
			starredOnly: favoriteOnly,
			workspace,
			pathStart,
			withoutDescription: true
		})
		return scripts.map((x) => x.path)
	}

	async function getFlows(
		favoriteOnly: boolean = false,
		workspace: string,
		folder: string | undefined
	) {
		if (!workspace) return []
		const pathStart = folder ? `f/${folder}` : undefined
		const flows = await FlowService.listFlows({
			starredOnly: favoriteOnly,
			workspace,
			pathStart,
			withoutDescription: true
		})
		return flows.map((x) => x.path)
	}

	// Fetch scripts+flows for a scope, cached and split by type so the same data
	// feeds both the preview list and the truncation count (no duplicate fetch).
	// The backend `pathStart` for a folder is an unbounded prefix (`f/team` also
	// matches `f/team2/...`), so restrict a folder fetch to the `f/{folder}/*`
	// subtree to mirror the MCP scope semantics.
	async function fetchScriptsAndFlows(
		workspace: string,
		favoriteOnly: boolean,
		folder: string | undefined
	): Promise<{ scripts: string[]; flows: string[] }> {
		const cacheKey = `${workspace}-${favoriteOnly}${folder ? `-${folder}` : ''}`
		const cached = runnablesCache.get(cacheKey)
		if (cached) return cached
		let [scripts, flows] = await Promise.all([
			getScripts(favoriteOnly, workspace, folder),
			getFlows(favoriteOnly, workspace, folder)
		])
		if (folder) {
			const pattern = [`f/${folder}/*`]
			scripts = scripts.filter((p) => matchesAnyPattern(p, pattern))
			flows = flows.filter((p) => matchesAnyPattern(p, pattern))
		}
		const result = { scripts, flows }
		runnablesCache.set(cacheKey, result)
		return result
	}

	async function loadAllScriptsAndFlows(workspace: string) {
		try {
			loadingRunnables = true
			const [scripts, flows] = await Promise.all([
				getScripts(false, workspace, undefined),
				getFlows(false, workspace, undefined)
			])
			allScripts = scripts
			allFlows = flows
		} finally {
			loadingRunnables = false
		}
	}

	// Load the preview list AND the exposed per-type counts for non-custom modes
	// from a single (cached) fetch — no duplicate requests. A sequence guard drops
	// stale async results when the scope changes faster than the fetches resolve.
	let runnablesSeq = 0
	async function loadRunnablesAndCounts() {
		const seq = ++runnablesSeq
		const ws = workspaceId
		if (!ws || selectedMode === 'custom') return
		if (selectedMode === 'folder' && selectedFolders.length === 0) {
			includedRunnables = []
			exposedScriptCount = 0
			exposedFlowCount = 0
			return
		}
		loadingRunnables = true
		try {
			let scripts: string[]
			let flows: string[]
			if (selectedMode === 'folder') {
				const perFolder = await Promise.all(
					selectedFolders.map((f) => fetchScriptsAndFlows(ws, false, f))
				)
				scripts = [...new Set(perFolder.flatMap((r) => r.scripts))]
				flows = [...new Set(perFolder.flatMap((r) => r.flows))]
			} else {
				const r = await fetchScriptsAndFlows(ws, selectedMode === 'favorites', undefined)
				scripts = r.scripts
				flows = r.flows
			}
			if (seq !== runnablesSeq) return
			includedRunnables = [...scripts, ...flows]
			exposedScriptCount = scripts.length
			exposedFlowCount = flows.length
		} finally {
			if (seq === runnablesSeq) loadingRunnables = false
		}
	}

	$effect(() => {
		// React to scope inputs (non-custom modes load list + counts together).
		selectedMode
		selectedFolders
		workspaceId
		loadRunnablesAndCounts()
	})

	// Load all scripts/flows for custom mode
	$effect(() => {
		if (selectedMode === 'custom' && workspaceId) {
			loadAllScriptsAndFlows(workspaceId)
		}
	})

	// One-shot: once allScripts/allFlows are loaded, split the
	// initial pattern text into known paths (selectedScripts/Flows) vs
	// remaining wildcards/unknowns (kept in pattern textbox).
	let initialSplitDone = $state(false)
	$effect(() => {
		if (initialSplitDone || selectedMode !== 'custom') return
		if (allScripts.length === 0 && allFlows.length === 0) return

		const scriptSet = new Set(allScripts)
		const flowSet = new Set(allFlows)

		const scriptParts = parsePatterns(customScriptPatterns)
		const knownScripts = scriptParts.filter((p) => scriptSet.has(p))
		const remainingScripts = scriptParts.filter((p) => !scriptSet.has(p))

		const flowParts = parsePatterns(customFlowPatterns)
		const knownFlows = flowParts.filter((p) => flowSet.has(p))
		const remainingFlows = flowParts.filter((p) => !flowSet.has(p))

		if (knownScripts.length > 0) {
			selectedScripts = [...new Set([...selectedScripts, ...knownScripts])]
			customScriptPatterns = remainingScripts.join(',')
		}
		if (knownFlows.length > 0) {
			selectedFlows = [...new Set([...selectedFlows, ...knownFlows])]
			customFlowPatterns = remainingFlows.join(',')
		}
		initialSplitDone = true
	})

	const warning = $derived(
		selectedMode === 'all'
			? 'Create your first scripts or flows to make them available via MCP.'
			: selectedMode === 'favorites'
				? `You do not have any favorite scripts or flows. You can favorite some scripts and flows to include them, or change the scope to "All scripts/flows" to include all your scripts and flows.`
				: `You do not have any scripts or flows in the selected folder(s).`
	)

	// How many scripts / flows the current scope would expose, per type, so we can
	// warn when it exceeds MCP_TOOL_FETCH_LIMIT (the server caps each type).
	let exposedScriptCount = $state<number | undefined>(undefined)
	let exposedFlowCount = $state<number | undefined>(undefined)

	// Mirror the backend's is_resource_allowed: `*`, an exact path, or an `x/*`
	// subtree (matching the folder itself or anything beneath it).
	function matchesAnyPattern(path: string, patterns: string[]): boolean {
		for (const p of patterns) {
			if (p === '*' || p === path) return true
			if (p.endsWith('/*')) {
				const prefix = p.slice(0, -2)
				if (path === prefix || (path.startsWith(prefix) && path[prefix.length] === '/')) {
					return true
				}
			}
		}
		return false
	}

	// Custom-mode counts are computed synchronously from the already-loaded
	// allScripts/allFlows plus the current selections/patterns — no extra fetch.
	$effect(() => {
		if (selectedMode !== 'custom') return
		const scriptPatterns = parsePatterns(customScriptPatterns)
		const flowPatterns = parsePatterns(customFlowPatterns)
		const scriptMatches = scriptPatterns.length
			? allScripts.filter((p) => matchesAnyPattern(p, scriptPatterns))
			: []
		const flowMatches = flowPatterns.length
			? allFlows.filter((p) => matchesAnyPattern(p, flowPatterns))
			: []
		exposedScriptCount = new Set([...selectedScripts, ...scriptMatches]).size
		exposedFlowCount = new Set([...selectedFlows, ...flowMatches]).size
	})

	const truncationWarning = $derived.by(() => {
		if (readOnly) return undefined
		const parts: string[] = []
		if ((exposedScriptCount ?? 0) > MCP_TOOL_FETCH_LIMIT)
			parts.push(`${exposedScriptCount} scripts`)
		if ((exposedFlowCount ?? 0) > MCP_TOOL_FETCH_LIMIT) parts.push(`${exposedFlowCount} flows`)
		if (parts.length === 0) return undefined
		return `This scope matches ${parts.join(' and ')}. Only the ${MCP_TOOL_FETCH_LIMIT} most recent of each type are exposed as MCP tools; the rest are omitted. Narrow the scope to avoid overloading the assistant's context.`
	})

	function selectAllScripts() {
		selectedScripts = [...allScripts]
	}
	function clearAllScripts() {
		selectedScripts = []
	}
	function selectAllFlows() {
		selectedFlows = [...allFlows]
	}
	function clearAllFlows() {
		selectedFlows = []
	}
	function selectAllEndpoints() {
		selectedEndpoints = [...visibleEndpointTools.map((e) => e.name)]
	}
	function clearAllEndpoints() {
		selectedEndpoints = []
	}
</script>

<div class="flex flex-col gap-4">
	<div>
		<span class="block mb-1 text-emphasis text-xs font-semibold">Scope</span>
		<ToggleButtonGroup bind:selected={selectedMode} allowEmpty={false}>
			{#snippet children({ item })}
				<ToggleButton
					{item}
					value="favorites"
					label="Favorites only"
					tooltip="Make only your favorite scripts and flows available as tools"
				/>
				<ToggleButton
					{item}
					value="all"
					label="All scripts/flows"
					tooltip="Make all your scripts and flows available as tools"
				/>
				<ToggleButton
					{item}
					value="folder"
					label="Folders"
					tooltip="Make all scripts and flows in the selected folders available as tools"
				/>
				<ToggleButton
					{item}
					value="custom"
					label="Custom"
					tooltip="Select exactly which scripts, flows, and endpoints to expose"
				/>
			{/snippet}
		</ToggleButtonGroup>
	</div>

	{#if truncationWarning}
		<Alert type="warning" size="xs" title="Too many tools for this scope">
			{truncationWarning}
		</Alert>
	{/if}

	{#if selectedMode === 'folder'}
		<div>
			<span class="block mb-1 text-emphasis text-xs font-semibold">Select Folders</span>
			{#if loadingFolders}
				<div class="text-xs text-primary">Loading folders...</div>
			{:else}
				<MultiSelect
					items={safeSelectItems(allFolders)}
					placeholder="Select folders"
					bind:value={selectedFolders}
				/>
			{/if}
		</div>
	{/if}

	<div>
		<span class="block mb-1 text-emphasis text-xs font-semibold">Hub scripts (optional)</span>
		{#if loadingApps}
			<div>Loading...</div>
		{:else if errorFetchApps}
			<div>Error fetching apps</div>
		{:else}
			<MultiSelect
				items={safeSelectItems(allApps)}
				placeholder="Select apps"
				bind:value={newMcpApps}
			/>
		{/if}
	</div>

	{#if selectedMode === 'custom'}
		{#if loadingRunnables}
			<div class="flex flex-col gap-2">
				<span class="block text-xs text-primary">Loading scripts and flows...</span>
				<div class="flex flex-wrap gap-1">
					<Badge rounded small color="dark-gray" baseClass="animate-skeleton">Loading...</Badge>
				</div>
			</div>
		{:else}
			{#snippet sectionHeader(label: string, selectAll: () => void, clearAll: () => void)}
				<div class="flex items-center justify-between">
					<span class="block text-xs font-semibold">{label}</span>
					<div class="flex gap-2">
						<Button size="xs2" onClick={selectAll}>Select All</Button>
						<Button size="xs2" onClick={clearAll}>Clear All</Button>
					</div>
				</div>
			{/snippet}

			<div class="flex flex-col gap-2">
				<div class="flex flex-col gap-2">
					{@render sectionHeader('Scripts', selectAllScripts, clearAllScripts)}
					{#if allScripts.length > 0}
						<MultiSelect
							items={safeSelectItems(allScripts)}
							placeholder="Select scripts"
							bind:value={selectedScripts}
						/>
					{:else}
						<p class="text-xs text-primary">No scripts available</p>
					{/if}
				</div>

				<div class="flex flex-col gap-2 mt-2">
					{@render sectionHeader('Flows', selectAllFlows, clearAllFlows)}
					{#if allFlows.length > 0}
						<MultiSelect
							items={safeSelectItems(allFlows)}
							placeholder="Select flows"
							bind:value={selectedFlows}
						/>
					{:else}
						<p class="text-xs text-primary">No flows available</p>
					{/if}
				</div>

				<div class="flex flex-col gap-2 mt-2">
					{@render sectionHeader('API Endpoints', selectAllEndpoints, clearAllEndpoints)}
					<MultiSelect
						items={safeSelectItems(visibleEndpointTools.map((e) => e.name))}
						placeholder="Select endpoints"
						bind:value={selectedEndpoints}
					/>
				</div>

				<div class="text-xs text-primary mt-2">
					Selected: {selectedScripts.length} scripts, {selectedFlows.length} flows, {selectedEndpoints.length}
					endpoints
				</div>

				<!-- Wildcard Patterns Section -->
				<div class="flex flex-col gap-2 mt-4 pt-4 border-t border-surface-hover">
					<div class="flex flex-col gap-2">
						<div class="flex items-center justify-between">
							<span class="block text-xs font-semibold">Script wildcard patterns</span>
							<Popover notClickable>
								{#snippet text()}
									<div class="text-xs max-w-xs">
										<p class="font-semibold mb-2">Add folder wildcards or complex patterns</p>
										<p class="mb-1"><b>Examples:</b></p>
										<ul class="list-disc ml-4 space-y-1">
											<li><code>f/folder/*</code> - all scripts/flows in folder</li>
											<li><code>f/folder1/*,f/folder2/*</code> - multiple folders</li>
											<li>Mix: <code>f/folder/*,f/specific/path</code></li>
										</ul>
										<p class="mt-2 text-xs text-secondary">
											Patterns are combined with individual selections above.
										</p>
									</div>
								{/snippet}
								<Button color="light" size="xs2" nonCaptureEvent startIcon={{ icon: InfoIcon }}>
									Pattern Help
								</Button>
							</Popover>
						</div>
						<TextInput
							inputProps={{ placeholder: 'e.g., f/outline/*,f/docs/*' }}
							bind:value={customScriptPatterns}
						/>
					</div>
					<div class="flex flex-col gap-2 mt-2">
						<div class="flex items-center justify-between">
							<span class="block text-xs font-semibold">Flow wildcard patterns</span>
						</div>
						<TextInput
							inputProps={{ placeholder: 'e.g., f/workflows/*' }}
							bind:value={customFlowPatterns}
						/>
					</div>
				</div>
			</div>
		{/if}
	{:else if selectedMode !== 'folder' || selectedFolders.length > 0}
		{#if loadingRunnables}
			<div class="flex flex-col gap-2">
				<span class="block text-xs text-primary"
					>Scripts & Flows that will be available via MCP</span
				>
				<div class="flex flex-wrap gap-1">
					<Badge rounded small color="dark-gray" baseClass="animate-skeleton">Loading...</Badge>
				</div>
			</div>
		{:else}
			<div class="flex flex-col gap-2">
				{#if !readOnly}
					<span class="block text-xs">Scripts & Flows that will be available via MCP</span>
					<div class="flex flex-wrap gap-1">
						{#if includedRunnables.length > 0 && includedRunnables.length <= 5}
							{#each includedRunnables as scriptOrFlow (scriptOrFlow)}
								<Badge rounded small color="blue">{scriptOrFlow}</Badge>
							{/each}
						{:else if includedRunnables.length > 0}
							{#each includedRunnables.slice(0, 3) as scriptOrFlow (scriptOrFlow)}
								<Badge rounded small color="blue">{scriptOrFlow}</Badge>
							{/each}
							<Badge rounded small color="dark-gray">
								+{includedRunnables.length - 3} more
							</Badge>
						{:else}
							<p class="text-xs text-primary">
								{warning}
							</p>
						{/if}
					</div>
				{:else}
					<p class="text-xs text-tertiary">
						Scripts and flows are hidden because this token is read-only.
					</p>
				{/if}

				<span class="block text-xs mt-2">API endpoint tools that will be available via MCP</span>
				<div class="flex flex-wrap gap-1">
					{#each visibleEndpointTools as endpoint (endpoint.name)}
						<Popover notClickable>
							{#snippet text()}
								<div class="flex flex-col gap-1">
									<div class="text-xs">{endpoint.description}</div>
									<div class="text-xs">
										{endpoint.method}
										{endpoint.path}
									</div>
								</div>
							{/snippet}
							<Badge rounded small color="green">{endpoint.name}</Badge>
						</Popover>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>

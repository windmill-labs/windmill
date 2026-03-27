<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { FlowService, FolderService, IntegrationService, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { mcpEndpointTools } from '$lib/mcpEndpointTools'
	import InfoIcon from 'lucide-svelte/icons/info'
	import { SvelteMap } from 'svelte/reactivity'

	interface Props {
		workspaceId: string
		scope: string
	}

	let { workspaceId, scope = $bindable() }: Props = $props()

	let selectedMode = $state<'favorites' | 'all' | 'folder' | 'custom'>('favorites')
	let selectedFolders = $state<string[]>([])
	let allFolders = $state<string[]>([])
	let loadingFolders = $state(false)
	let selectedScripts = $state<string[]>([])
	let selectedFlows = $state<string[]>([])
	let selectedEndpoints = $state<string[]>([])
	let customScriptPatterns = $state<string>('')
	let customFlowPatterns = $state<string>('')
	let newMcpApps = $state<string[]>([])

	let allScripts = $state<string[]>([])
	let allFlows = $state<string[]>([])
	let allApps = $state<string[]>([])
	let loadingApps = $state(false)
	let errorFetchApps = $state(false)
	let loadingRunnables = $state(false)
	let includedRunnables = $state<string[]>([])

	let runnablesCache = new SvelteMap<string, string[]>()

	function parsePatterns(input: string): string[] {
		return input
			.split(',')
			.map((p) => p.trim())
			.filter((p) => p.length > 0)
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
			if (folderPaths.length > 0) {
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
		if (selectedMode === 'folder') {
			loadFolderNames()
		} else {
			selectedFolders = []
		}
	})

	async function loadFolderNames() {
		if (allFolders.length > 0) return
		try {
			loadingFolders = true
			const excludedFolders = ['app_groups', 'app_custom', 'app_themes']
			allFolders = (
				await FolderService.listFolderNames({
					workspace: $workspaceStore!
				})
			).filter((x) => !excludedFolders.includes(x))
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

	async function getScriptsAndFlows(
		favoriteOnly: boolean = false,
		workspace: string,
		folder: string | undefined
	) {
		const cacheKey = `${workspace}-${favoriteOnly}${folder ? `-${folder}` : ''}`
		if (runnablesCache.has(cacheKey)) {
			includedRunnables = runnablesCache.get(cacheKey) || []
			return
		}
		try {
			loadingRunnables = true
			const [scripts, flows] = await Promise.all([
				getScripts(favoriteOnly, workspace, folder),
				getFlows(favoriteOnly, workspace, folder)
			])
			const combined = [...scripts, ...flows]
			runnablesCache.set(cacheKey, combined)
			includedRunnables = combined
		} finally {
			loadingRunnables = false
		}
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

	// Load runnables based on mode
	$effect(() => {
		if (workspaceId) {
			if (selectedMode === 'folder' && selectedFolders.length > 0) {
				loadRunnablesForFolders(workspaceId, selectedFolders)
			} else {
				getScriptsAndFlows(selectedMode === 'favorites', workspaceId, undefined)
			}
		}
	})

	async function getCachedRunnables(workspace: string, folder: string): Promise<string[]> {
		const cacheKey = `${workspace}-false-${folder}`
		if (runnablesCache.has(cacheKey)) {
			return runnablesCache.get(cacheKey) || []
		}
		const [scripts, flows] = await Promise.all([
			getScripts(false, workspace, folder),
			getFlows(false, workspace, folder)
		])
		const combined = [...scripts, ...flows]
		runnablesCache.set(cacheKey, combined)
		return combined
	}

	async function loadRunnablesForFolders(workspace: string, folders: string[]) {
		try {
			loadingRunnables = true
			const results = await Promise.all(folders.map((f) => getCachedRunnables(workspace, f)))
			includedRunnables = [...new Set(results.flat())]
		} finally {
			loadingRunnables = false
		}
	}

	// Load all scripts/flows for custom mode
	$effect(() => {
		if (selectedMode === 'custom' && workspaceId) {
			loadAllScriptsAndFlows(workspaceId)
		}
	})

	const warning = $derived(
		selectedMode === 'all'
			? 'Create your first scripts or flows to make them available via MCP.'
			: selectedMode === 'favorites'
				? `You do not have any favorite scripts or flows. You can favorite some scripts and flows to include them, or change the scope to "All scripts/flows" to include all your scripts and flows.`
				: `You do not have any scripts or flows in the selected folder(s).`
	)

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
		selectedEndpoints = [...mcpEndpointTools.map((e) => e.name)]
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
						items={safeSelectItems(mcpEndpointTools.map((e) => e.name))}
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

				<span class="block text-xs mt-2">API endpoint tools that will be available via MCP</span>
				<div class="flex flex-wrap gap-1">
					{#each mcpEndpointTools as endpoint (endpoint.name)}
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

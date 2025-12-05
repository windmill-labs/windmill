<script lang="ts">
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { Badge, Button } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import Popover from '../Popover.svelte'
	import {
		FlowService,
		IntegrationService,
		ScriptService,
		UserService,
		type NewToken
	} from '$lib/gen'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopeSelector from './ScopeSelector.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import FolderPicker from '../FolderPicker.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import { mcpEndpointTools } from '$lib/mcpEndpointTools'
	import InfoIcon from 'lucide-svelte/icons/info'

	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		newTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated: (token: string) => void
		displayCreateToken?: boolean
	}

	let {
		showMcpMode = false,
		defaultNewTokenWorkspace,
		scopes,
		onTokenCreated,
		newTokenLabel = $bindable(undefined)
	}: Props = $props()

	// MCP clients do not allow names longer than 60 characters, here we use 55 because final tool name server side will add ~5 characters
	const MAX_PATH_LENGTH = 55

	let newToken = $state<string | undefined>(undefined)
	let newMcpToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(defaultNewTokenWorkspace)
	let newMcpApps = $state<string[]>([])
	let mcpCreationMode = $state(false)
	let newMcpScope = $state('favorites')
	let loadingApps = $state(false)
	let errorFetchApps = $state(false)
	let allApps = $state<string[]>([])
	let loadingRunnables = $state(false)
	let includedRunnables = $state<string[]>([])
	let selectedFolder = $state<string>('')

	// Granular scope selection
	let selectedScripts = $state<string[]>([])
	let selectedFlows = $state<string[]>([])
	let selectedEndpoints = $state<string[]>([])
	let allScripts = $state<string[]>([])
	let allFlows = $state<string[]>([])

	let runnablesCache = new Map<string, string[]>()

	let customScopes = $state<string[]>([])
	let showCustomScopes = $state(false)

	// Wildcard pattern inputs for custom scope
	let customScriptPatterns = $state<string>('')
	let customFlowPatterns = $state<string>('')

	function ensureCurrentWorkspaceIncluded(
		workspacesList: UserWorkspace[],
		currentWorkspace: string | undefined
	) {
		if (!currentWorkspace) {
			return workspacesList
		}
		const hasCurrentWorkspace = workspacesList.some((w) => w.id === currentWorkspace)
		if (hasCurrentWorkspace) {
			return workspacesList
		}
		return [{ id: currentWorkspace, name: currentWorkspace }, ...workspacesList]
	}

	function parsePatterns(input: string): string[] {
		return input
			.split(',')
			.map((p) => p.trim())
			.filter((p) => p.length > 0)
	}

	// Clear pattern inputs when MCP mode is disabled OR when not in custom scope
	$effect(() => {
		if (!mcpCreationMode || newMcpScope !== 'custom') {
			customScriptPatterns = ''
			customFlowPatterns = ''
		}
	})

	async function createToken(mcpMode: boolean = false): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			let tokenScopes = scopes
			if (mcpMode) {
				if (newMcpScope === 'custom') {
					// Granular scope format - combine individual selections with wildcard patterns
					tokenScopes = []

					// Scripts: combine individual selections with patterns
					let scriptPaths = [...selectedScripts]
					if (customScriptPatterns.trim()) {
						scriptPaths.push(...parsePatterns(customScriptPatterns))
					}
					if (scriptPaths.length > 0) {
						tokenScopes.push(`mcp:scripts:${scriptPaths.join(',')}`)
					}

					// Flows: combine individual selections with patterns
					let flowPaths = [...selectedFlows]
					if (customFlowPatterns.trim()) {
						flowPaths.push(...parsePatterns(customFlowPatterns))
					}
					if (flowPaths.length > 0) {
						tokenScopes.push(`mcp:flows:${flowPaths.join(',')}`)
					}

					// Endpoints: no wildcard support needed
					if (selectedEndpoints.length > 0) {
						tokenScopes.push(`mcp:endpoints:${selectedEndpoints.join(',')}`)
					}
				} else if (newMcpScope === 'folder') {
					const folderPath = `f/${selectedFolder}/*`
					tokenScopes = [`mcp:scripts:${folderPath}`, `mcp:flows:${folderPath}`, `mcp:endpoints:*`]
				} else {
					tokenScopes = [`mcp:${newMcpScope}`]
				}
				if (newMcpApps.length > 0) {
					tokenScopes.push(`mcp:hub:${newMcpApps.join(',')}`)
				}
			} else if (showCustomScopes && customScopes.length > 0) {
				tokenScopes = customScopes
			}

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: mcpMode ? newTokenWorkspace || $workspaceStore : newTokenWorkspace
				} as NewToken
			})

			if (mcpMode) {
				newMcpToken = `${createdToken}`
			} else {
				newToken = `${createdToken}`
			}

			onTokenCreated(newToken ?? newMcpToken ?? '')
			mcpCreationMode = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const mcpBaseUrl = $derived(`${window.location.origin}/api/mcp/w/${newTokenWorkspace}/sse?token=`)

	const warning = $derived(
		newMcpScope === 'all'
			? 'Create your first scripts or flows to make them available via MCP.'
			: newMcpScope === 'favorites'
				? `You do not have any favorite scripts or flows. You can favorite some scripts and flows to include them, or change the scope to "All scripts/flows" to include all your scripts and flows.`
				: `You do not have any scripts or flows in the selected folder.`
	)
	const longPathRunnables = $derived(
		includedRunnables.filter((path) => path.length > MAX_PATH_LENGTH)
	)
	const validRunnables = $derived(
		includedRunnables.filter((path) => path.length <= MAX_PATH_LENGTH)
	)
	const longPathWarning = $derived(
		longPathRunnables.length > 0
			? `${longPathRunnables.length} script(s)/flow(s) have paths longer than 60 characters and will be excluded from MCP tools. Consider shortening the paths: ${longPathRunnables.slice(0, 3).join(', ')}${longPathRunnables.length > 3 ? ` and ${longPathRunnables.length - 3} more` : ''}`
			: ''
	)

	$effect(() => {
		if (mcpCreationMode) {
			getAllApps()
		} else {
			newMcpApps = []
		}
	})

	async function getAllApps() {
		if (allApps.length > 0) {
			return
		}
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

	async function getScripts(
		favoriteOnly: boolean = false,
		workspace: string,
		folder: string | undefined
	) {
		if (!workspace) {
			return []
		}
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
		if (!workspace) {
			return []
		}
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

	$effect(() => {
		if (mcpCreationMode) {
			const workspace = newTokenWorkspace || $workspaceStore
			if (workspace) {
				const folderParam = selectedFolder.length > 0 ? selectedFolder : undefined
				getScriptsAndFlows(newMcpScope === 'favorites', workspace, folderParam)
			}
		} else {
			includedRunnables = []
		}
	})

	$effect(() => {
		if (mcpCreationMode && newMcpScope !== 'folder') {
			selectedFolder = ''
		}
	})

	$effect(() => {
		if (mcpCreationMode && newMcpScope === 'custom') {
			const workspace = newTokenWorkspace || $workspaceStore
			if (workspace) {
				loadAllScriptsAndFlows(workspace)
			}
		}
	})

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

<div>
	<div class="p-4 rounded-md mb-6 min-w-min bg-surface-tertiary">
		<h3 class="pb-2 font-semibold text-emphasis text-sm">Add a new token</h3>

		{#if showMcpMode}
			<div
				class="mb-4 flex flex-row flex-shrink-0"
				use:triggerableByAI={{
					id: 'account-settings-create-mcp-token',
					description: 'Create a new MCP token to authenticate to the Windmill API'
				}}
			>
				<Toggle
					on:change={(e) => {
						mcpCreationMode = e.detail
						if (e.detail) {
							newTokenLabel = 'MCP token'
							newTokenExpiration = undefined
							newTokenWorkspace = $workspaceStore
						} else {
							newTokenLabel = undefined
							newTokenExpiration = undefined
							newTokenWorkspace = defaultNewTokenWorkspace
						}
					}}
					checked={mcpCreationMode}
					options={{
						right: 'Generate MCP URL',
						rightTooltip:
							'Generate a new MCP URL to make your scripts, flows, and API endpoints available as tools through your LLM clients.',
						rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mcp'
					}}
					size="xs"
				/>
			</div>
		{/if}

		{#if scopes != undefined}
			<div class="mb-4">
				<span class="block mb-1 text-emphasis text-xs font-semibold">Scope</span>
				{#each scopes as scope}
					<TextInput inputProps={{ disabled: true }} value={scope} class="mb-2 w-full" />
				{/each}
			</div>
		{/if}

		{#if !mcpCreationMode && (!scopes || scopes.length === 0)}
			<div class="flex flex-col gap-2">
				<Toggle
					checked={showCustomScopes}
					on:change={(e) => {
						showCustomScopes = e.detail
					}}
					options={{
						right: 'Limit token permissions',
						rightTooltip:
							'By default, tokens have full API access. Enable this to restrict the token to specific scopes.'
					}}
					size="xs"
				/>
				{#if showCustomScopes}
					<ScopeSelector bind:selectedScopes={customScopes} />
				{/if}
			</div>
		{/if}

		<div class="mt-2 grid grid-cols-1 md:grid-cols-2 gap-4">
			{#if mcpCreationMode}
				<div class="col-span-2">
					<span class="block mb-1 text-emphasis text-xs font-semibold">Scope</span>
					<ToggleButtonGroup bind:selected={newMcpScope} allowEmpty={false}>
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
								label="Folder"
								tooltip="Make all scripts and flows in the selected folder available as tools"
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

				{#if newMcpScope === 'folder'}
					<div>
						<span class="block mb-1 text-emphasis text-xs font-semibold">Select Folder</span>
						<FolderPicker bind:folderName={selectedFolder} />
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

				<div>
					<span class="block mb-1 text-emphasis text-xs font-semibold">Workspace</span>
					<select bind:value={newTokenWorkspace} disabled={workspaces.length === 1} class="w-full">
						{#each workspaces as workspace}
							<option value={workspace.id}>{workspace.name}</option>
						{/each}
					</select>
				</div>
			{/if}

			<div>
				<span class="block mb-1 text-emphasis text-xs font-semibold"
					>Label <span class="text-xs text-primary">(optional)</span></span
				>
				<TextInput inputProps={{ type: 'text' }} bind:value={newTokenLabel} class="w-full" />
			</div>

			{#if !mcpCreationMode}
				<div>
					<span class="block mb-1 text-xs text-emphasis font-semibold"
						>Expires In <span class="text-xs text-primary">(optional)</span></span
					>
					<Select
						bind:value={newTokenExpiration}
						placeholder="No expiration"
						inputClass="w-full"
						items={[
							{ label: 'No expiration', value: undefined },
							{ label: '15 minutes', value: 15 * 60 },
							{ label: '30 minutes', value: 30 * 60 },
							{ label: '1 hour', value: 1 * 60 * 60 },
							{ label: '1 day', value: 1 * 24 * 60 * 60 },
							{ label: '7 days', value: 7 * 24 * 60 * 60 },
							{ label: '30 days', value: 30 * 24 * 60 * 60 },
							{ label: '90 days', value: 90 * 24 * 60 * 60 }
						]}
					/>
				</div>
			{/if}
			{#if mcpCreationMode && newMcpScope === 'custom'}
				{#if loadingRunnables}
					<div class="flex flex-col gap-2 col-span-2 pr-4">
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
								<Button size="xs2" on:click={selectAll}>Select All</Button>
								<Button size="xs2" on:click={clearAll}>Clear All</Button>
							</div>
						</div>
					{/snippet}

					<div class="flex flex-col gap-2 col-span-2 pr-4">
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
			{:else if mcpCreationMode && (newMcpScope !== 'folder' || selectedFolder.length > 0)}
				{#if loadingRunnables}
					<div class="flex flex-col gap-2 col-span-2 pr-4">
						<span class="block text-xs text-primary"
							>Scripts & Flows that will be available via MCP</span
						>
						<div class="flex flex-wrap gap-1">
							<Badge rounded small color="dark-gray" baseClass="animate-skeleton">Loading...</Badge>
						</div>
					</div>
				{:else}
					<div class="flex flex-col gap-2 col-span-2 pr-4">
						{#if longPathWarning}
							<Alert type="warning" title="Some paths are too long" size="xs">
								{longPathWarning}
							</Alert>
						{/if}
						<span class="block text-xs">Scripts & Flows that will be available via MCP</span>
						<div class="flex flex-wrap gap-1">
							{#if validRunnables.length > 0 && validRunnables.length <= 5}
								{#each validRunnables as scriptOrFlow}
									<Badge rounded small color="blue">{scriptOrFlow}</Badge>
								{/each}
							{:else if validRunnables.length > 0}
								{#each validRunnables.slice(0, 3) as scriptOrFlow}
									<Badge rounded small color="blue">{scriptOrFlow}</Badge>
								{/each}
								<Badge rounded small color="dark-gray">
									+{validRunnables.length - 3} more
								</Badge>
							{:else}
								<p class="text-xs text-primary">
									{warning}
								</p>
							{/if}
						</div>

						<span class="block text-xs mt-2">API endpoint tools that will be available via MCP</span
						>
						<div class="flex flex-wrap gap-1">
							{#each mcpEndpointTools as endpoint}
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

		<div class="mt-4 flex justify-end gap-2 flex-row">
			<Button
				on:click={() => {
					mcpCreationMode = false
				}}
				variant="default"
			>
				Cancel
			</Button>
			<Button
				on:click={() => createToken(mcpCreationMode)}
				disabled={mcpCreationMode &&
					(newTokenWorkspace == undefined ||
						(newMcpScope === 'folder' && !selectedFolder) ||
						(newMcpScope === 'custom' &&
							selectedScripts.length === 0 &&
							selectedFlows.length === 0 &&
							selectedEndpoints.length === 0 &&
							!customScriptPatterns.trim() &&
							!customFlowPatterns.trim()))}
				variant="accent"
			>
				New token
			</Button>
		</div>
	</div>

	{#if newToken}
		<TokenDisplay token={newToken} />
	{/if}

	{#if newMcpToken}
		<TokenDisplay token={newMcpToken} mcpUrl={`${mcpBaseUrl}${newMcpToken}`} />
	{/if}
</div>

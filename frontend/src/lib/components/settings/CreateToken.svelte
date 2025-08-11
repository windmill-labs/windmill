<script lang="ts">
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { Badge, Button } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { FlowService, IntegrationService, ScriptService, UserService, type NewToken } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopeSelector from './ScopeSelector.svelte'
	import Alert from '../common/alert/Alert.svelte'

	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		newTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated?: (token: string) => void
		displayCreateToken?: boolean
	}

	let {
		showMcpMode = false,
		defaultNewTokenWorkspace,
		scopes,
		onTokenCreated,
		newTokenLabel = $bindable(undefined)
	}: Props = $props()

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

	let customScopes = $state<string[]>([])
	let showCustomScopes = $state(false)

	const dispatch = createEventDispatcher()

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

	async function createToken(mcpMode: boolean = false): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			let tokenScopes = scopes
			if (mcpMode) {
				tokenScopes = [`mcp:${newMcpScope}`]
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

			dispatch('tokenCreated', newToken ?? newMcpToken)
			if (onTokenCreated) {
				onTokenCreated(newToken ?? newMcpToken ?? '')
			}
			mcpCreationMode = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const mcpBaseUrl = $derived(`${window.location.origin}/api/mcp/w/${newTokenWorkspace}/sse?token=`)
	const warning = $derived(newMcpScope === 'favorites' ? `You do not have any favorite scripts or flows. You can favorite some scripts and flows to include them, or change the scope to "All scripts/flows" to include all your scripts and flows.` : 'Create your first scripts or flows to make them available via MCP.')
	const noScriptsOrFlowsAvailableWarning = $derived(includedRunnables.length === 0 ? warning : '')

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

	async function getScripts(favoriteOnly: boolean = false) {
		const workspace = newTokenWorkspace || $workspaceStore
		if (!workspace) {
			return []
		}
		const scripts = await ScriptService.listScripts({
			starredOnly: favoriteOnly,
			workspace
		})
		return scripts.map((x) => x.path)
	}

	async function getFlows(favoriteOnly: boolean = false) {
		const workspace = newTokenWorkspace || $workspaceStore
		if (!workspace) {
			return []
		}
		const flows = await FlowService.listFlows({
			starredOnly: favoriteOnly,
			workspace
		})
		return flows.map((x) => x.path)
	}

	async function getScriptsAndFlows(favoriteOnly: boolean = false) {
		try {
			loadingRunnables = true
			const [scripts, flows] = await Promise.all([getScripts(favoriteOnly), getFlows(favoriteOnly)])
			includedRunnables = [...scripts, ...flows]
		} finally {
			loadingRunnables = false
		}
	}

	$effect(() => {
		if (mcpCreationMode) {
			getScriptsAndFlows(newMcpScope === 'favorites')
		} else {
			includedRunnables = []
		}
	})
</script>

<div>
	<div class="py-3 px-3 border rounded-md mb-6 bg-surface-secondary min-w-min">
		<h3 class="pb-3 font-semibold">Add a new token</h3>

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
							'Generate a new MCP URL to make your scripts and flows available as tools through your LLM clients.',
						rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mcp'
					}}
					size="xs"
				/>
			</div>
		{/if}

		{#if scopes != undefined}
			<div class="mb-4">
				<span class="block mb-1">Scope</span>
				{#each scopes as scope}
					<input disabled type="text" value={scope} class="mb-2 w-full" />
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
				<div>
					<span class="block mb-1">Scope</span>
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
						{/snippet}
					</ToggleButtonGroup>
				</div>

				<div>
					<span class="block mb-1">Hub scripts (optional)</span>
					{#if loadingApps}
						<div>Loading...</div>
					{:else if errorFetchApps}
						<div>Error fetching apps</div>
					{:else}
						<MultiSelect
							items={safeSelectItems(allApps)}
							placeholder="Select apps"
							bind:value={newMcpApps}
							class="!bg-surface"
						/>
					{/if}
				</div>

				<div>
					<span class="block mb-1">Workspace</span>
					<select bind:value={newTokenWorkspace} disabled={workspaces.length === 1} class="w-full">
						{#each workspaces as workspace}
							<option value={workspace.id}>{workspace.name}</option>
						{/each}
					</select>
				</div>
			{/if}

			<div>
				<span class="block mb-1">Label <span class="text-xs text-tertiary">(optional)</span></span>
				<input type="text" bind:value={newTokenLabel} class="w-full" />
			</div>

			{#if !mcpCreationMode}
				<div>
					<span class="block mb-1"
						>Expires In <span class="text-xs text-tertiary">(optional)</span></span
					>
					<select bind:value={newTokenExpiration} class="w-full">
						<option value={undefined}>No expiration</option>
						<option value={15 * 60}>15m</option>
						<option value={30 * 60}>30m</option>
						<option value={1 * 60 * 60}>1h</option>
						<option value={1 * 24 * 60 * 60}>1d</option>
						<option value={7 * 24 * 60 * 60}>7d</option>
						<option value={30 * 24 * 60 * 60}>30d</option>
						<option value={90 * 24 * 60 * 60}>90d</option>
					</select>
				</div>
			{:else if loadingRunnables}
				<div class="flex flex-col gap-2 col-span-2 pr-4">
					<span class="block text-xs text-tertiary">Loading scripts & flows...</span>
				</div>
			{:else}
				<div class="flex flex-col gap-2 col-span-2 pr-4">
					{#if noScriptsOrFlowsAvailableWarning}
					<Alert type="info" title="No scripts or flows available" size="xs">
						{noScriptsOrFlowsAvailableWarning}
					</Alert>
					{:else}
						<span class="block text-xs text-tertiary">Scripts & Flows that will be available via MCP</span>
						<div class="flex flex-wrap gap-1">

						{#if includedRunnables.length <= 5}
							{#each includedRunnables as scriptOrFlow}
								<Badge rounded small color="blue">{scriptOrFlow}</Badge>
							{/each}
						{:else}
							{#each includedRunnables.slice(0, 3) as scriptOrFlow}
								<Badge rounded small color="blue">{scriptOrFlow}</Badge>
							{/each}
							<Badge 
								rounded 
								small 
								color="dark-gray" 
							>
								+{includedRunnables.length - 3} more
							</Badge>
							{/if}
						</div>
					{/if}
				</div>
			{/if}

		</div>

		<div class="mt-4 flex justify-end gap-2 flex-row">
			<Button
				on:click={() => {
					mcpCreationMode = false
				}}
			>
				Cancel
			</Button>
			<Button
				on:click={() => createToken(mcpCreationMode)}
				disabled={mcpCreationMode && newTokenWorkspace == undefined}
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

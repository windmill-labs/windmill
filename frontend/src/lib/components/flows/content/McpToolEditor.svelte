<script module lang="ts">
	import { get } from 'svelte/store'
	import { workspaceStore, userStore } from '$lib/stores'
	import { ResourceService } from '$lib/gen'
	import { createCache } from '$lib/utils'

	let loadToolsCached = createCache(
		({ workspace, path }: { workspace?: string; path?: string; refreshCount?: number }) =>
			workspace && path && get(userStore)
				? ResourceService.getMcpTools({ workspace, path })
				: undefined,
		{
			initial: { workspace: get(workspaceStore), path: undefined, refreshCount: 0 },
			invalidateMs: 1000 * 60
		} // Cache for 60 seconds
	)
</script>

<script lang="ts">
	import type { McpTool } from '../agentToolUtils'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { untrack } from 'svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { OauthService, VariableService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Path from '$lib/components/Path.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { onDestroy } from 'svelte'

	interface Props {
		tool: McpTool
		noEditor: boolean
	}

	let { tool = $bindable() }: Props = $props()

	// OAuth state
	let showOAuthForm = $state(false)
	let serverUrl = $state('')
	let discoveryResult = $state<{
		scopes_supported?: string[]
		authorization_endpoint: string
		token_endpoint: string
		registration_endpoint?: string
		supports_dynamic_registration: boolean
	} | null>(null)
	let selectedScopes = $state<string[]>([])
	let resourceName = $state('')
	let resourcePath = $state('')
	let pathError = $state('')
	let oauthStatus = $state<'idle' | 'discovering' | 'discovered' | 'connecting'>('idle')
	let oauthError = $state<string | null>(null)

	// Discover OAuth metadata from MCP server
	async function discoverOAuth() {
		oauthStatus = 'discovering'
		oauthError = null
		try {
			const response = await fetch(`/api/mcp/oauth/discover`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ mcp_server_url: serverUrl })
			})
			if (!response.ok) {
				const err = await response.json()
				throw new Error(err.message || 'Discovery failed')
			}
			discoveryResult = await response.json()
			selectedScopes = discoveryResult?.scopes_supported ?? []
			// Generate default resource name from URL
			try {
				const urlObj = new URL(serverUrl)
				resourceName = urlObj.hostname.replace(/\./g, '_')
			} catch {
				resourceName = 'mcp_server'
			}
			oauthStatus = 'discovered'
		} catch (e: any) {
			oauthError = e.message || 'Failed to discover OAuth settings'
			oauthStatus = 'idle'
		}
	}

	// Start OAuth popup flow
	function startOAuth() {
		const url = new URL(`/api/mcp/oauth/start`, window.location.origin)
		url.searchParams.set('mcp_server_url', serverUrl)
		url.searchParams.set('scopes', selectedScopes.join(','))

		window.addEventListener('message', handleOAuthMessage)
		window.addEventListener('storage', handleStorageEvent)
		window.open(url.toString(), '_blank', 'popup=true')
		oauthStatus = 'connecting'
	}

	// Handle postMessage from OAuth popup
	function handleOAuthMessage(event: MessageEvent) {
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) return

		if (event.data.type === 'MCP_CONNECTED') {
			window.removeEventListener('message', handleOAuthMessage)
			window.removeEventListener('storage', handleStorageEvent)
			createMcpResource(event.data)
		} else if (event.data.type === 'MCP_ERROR') {
			window.removeEventListener('message', handleOAuthMessage)
			window.removeEventListener('storage', handleStorageEvent)
			oauthError = event.data.error
			oauthStatus = 'discovered'
		}
	}

	// Handle localStorage fallback for popup communication
	function handleStorageEvent(event: StorageEvent) {
		if (event.key === 'mcp-oauth-callback') {
			try {
				const data = JSON.parse(event.newValue || '{}')
				localStorage.removeItem('mcp-oauth-callback')
				if (data.type === 'MCP_CONNECTED') {
					createMcpResource(data)
				}
			} catch (_) {
				// Silently ignore invalid callback data
			}
			window.removeEventListener('storage', handleStorageEvent)
		}
	}

	// Create account, variable, and resource after OAuth success
	async function createMcpResource(data: {
		access_token: string
		refresh_token?: string
		expires_in?: number
		mcp_server_url: string
	}) {
		try {
			// 1. Create account for token refresh (if we have expiration info)
			let accountId: number | undefined
			if (data.expires_in && data.refresh_token) {
				const accountIdStr = await OauthService.createAccount({
					workspace: $workspaceStore!,
					requestBody: {
						refresh_token: data.refresh_token,
						expires_in: data.expires_in,
						client: 'mcp',
						mcp_server_url: data.mcp_server_url
					}
				})
				accountId = Number(accountIdStr)
			}

			// 2. Create secret variable for access token
			await VariableService.createVariable({
				workspace: $workspaceStore!,
				requestBody: {
					path: resourcePath,
					value: data.access_token,
					is_secret: true,
					is_oauth: true,
					account: accountId,
					description: `MCP OAuth token for ${data.mcp_server_url}`
				}
			})

			// 3. Create MCP resource
			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type: 'mcp',
					path: resourcePath,
					value: {
						name: resourceName,
						url: data.mcp_server_url,
						token: `$var:${resourcePath}`
					},
					description: `MCP server connected via OAuth`
				}
			})

			// 4. Select the new resource
			tool.value.resource_path = resourcePath
			tool.summary = `MCP: ${resourceName}`

			// 5. Reset and close form
			resetOAuthForm()
			sendUserToast('Connected to MCP server')
		} catch (e: any) {
			oauthError = e.body?.message || e.message || 'Failed to create resource'
			oauthStatus = 'discovered'
		}
	}

	// Reset OAuth form to initial state
	function resetOAuthForm() {
		showOAuthForm = false
		oauthStatus = 'idle'
		serverUrl = ''
		discoveryResult = null
		selectedScopes = []
		resourceName = ''
		resourcePath = ''
		oauthError = null
	}

	// Cleanup event listeners on component destroy
	onDestroy(() => {
		window.removeEventListener('message', handleOAuthMessage)
		window.removeEventListener('storage', handleStorageEvent)
	})

	let refreshCount = $state(0)

	let tools = usePromise(
		async () =>
			await loadToolsCached({
				workspace: $workspaceStore!,
				path: tool.value.resource_path,
				refreshCount
			}),
		{ loadInit: false, clearValueOnRefresh: false }
	)

	// Options for the multiselect
	let toolOptions = $derived(safeSelectItems((tools.value ?? []).map((t) => t.name)))

	// Watch for resource_path changes and refresh tools
	$effect(() => {
		// Track reactive dependencies
		tool.value.resource_path
		$workspaceStore
		refreshCount
		// Trigger refresh when resource_path or workspace changes
		untrack(() => {
			if (tool.value.resource_path?.length > 0) {
				tools.refresh()
			}
		})
	})

	$effect(() => {
		if (!tool.value.include_tools) {
			tool.value.include_tools = []
		}
		if (!tool.value.exclude_tools) {
			tool.value.exclude_tools = []
		}
	})

	$effect(() => {
		if (tool.value.resource_path?.length > 0 && tool.summary?.length === 0) {
			tool.summary = `MCP: ${tool.value.resource_path}`
		}
	})
</script>

<div class="flex flex-col gap-4 p-4">
	<!-- Explanatory Section -->
	<Alert type="info" title="MCP Client Configuration">
		{#snippet children()}
			<p class="mb-2 text-sm">
				MCP clients allow AI agents to access and execute a list of tools made available by an MCP
				server.
				<br />
				Choose an MCP resource to make its tools available to the agent.
				<br />
				<br />
				<strong>Note:</strong> Only HTTP streamable MCP servers are supported.
			</p>
		{/snippet}
	</Alert>

	<!-- Resource Path Section -->
	<div class="w-full">
		<Label label="MCP Resource">
			<ResourcePicker resourceType="mcp" bind:value={tool.value.resource_path} />
		</Label>
	</div>

	<!-- OAuth Connect Button/Form -->
	{#if !showOAuthForm}
		<Button size="xs" color="light" on:click={() => (showOAuthForm = true)}>
			Connect with OAuth
		</Button>
	{:else}
		<div class="border rounded p-4 bg-surface-secondary flex flex-col gap-4">
			<div class="flex justify-between items-center">
				<span class="font-semibold text-sm">Connect MCP Server with OAuth</span>
				<Button size="xs" color="light" on:click={resetOAuthForm}>Cancel</Button>
			</div>

			<!-- Server URL -->
			<Label label="MCP Server URL">
				<input
					type="url"
					bind:value={serverUrl}
					placeholder="https://mcp.example.com"
					class="text-sm w-full"
					disabled={oauthStatus === 'connecting'}
				/>
			</Label>

			{#if oauthStatus === 'idle'}
				<Button size="sm" on:click={discoverOAuth} disabled={!serverUrl}>
					Discover OAuth Settings
				</Button>
			{:else if oauthStatus === 'discovering'}
				<div class="text-sm text-secondary">Discovering OAuth settings...</div>
			{:else if oauthStatus === 'discovered' && discoveryResult}
				<!-- Show discovery results -->
				<div class="text-xs text-green-600 dark:text-green-400">
					&#10003; OAuth supported
					{#if discoveryResult.supports_dynamic_registration}
						(Dynamic Client Registration available)
					{/if}
				</div>

				<!-- Scope selection (checkboxes) -->
				{#if discoveryResult.scopes_supported && discoveryResult.scopes_supported.length > 0}
					<Label label="Select Scopes">
						<div class="flex flex-col flex-wrap gap-2">
							{#each discoveryResult.scopes_supported as scope}
								<label class="flex flex-row items-center gap-2 text-xs cursor-pointer">
									<input
										type="checkbox"
										checked={selectedScopes.includes(scope)}
										onchange={(e) => {
											const target = e.target as HTMLInputElement
											if (target.checked) {
												selectedScopes = [...selectedScopes, scope]
											} else {
												selectedScopes = selectedScopes.filter((s) => s !== scope)
											}
										}}
										class="!w-4 !h-4"
									/>
									{scope}
								</label>
							{/each}
						</div>
					</Label>
				{/if}

				<!-- Resource name -->
				<Label label="Resource Name">
					<input
						type="text"
						bind:value={resourceName}
						placeholder="my-mcp-server"
						class="text-sm w-full"
					/>
				</Label>

				<!-- Resource path -->
				<Path
					bind:path={resourcePath}
					bind:error={pathError}
					initialPath=""
					namePlaceholder={resourceName}
					kind="resource"
				/>

				<!-- Connect button -->
				<Button size="sm" on:click={startOAuth} disabled={!resourcePath || pathError !== ''}>
					Connect with OAuth
				</Button>
			{:else if oauthStatus === 'connecting'}
				<div class="text-sm text-secondary">Complete authentication in popup window...</div>
			{/if}

			{#if oauthError}
				<div class="text-xs text-red-600 dark:text-red-400">{oauthError}</div>
			{/if}
		</div>
	{/if}

	{#if tool.value.resource_path?.length > 0}
		<!-- Summary Section -->
		<div class="w-full">
			<Label label="Summary">
				<input
					type="text"
					bind:value={tool.summary}
					placeholder="e.g., GitHub MCP"
					class="text-sm w-full"
				/>
			</Label>
		</div>

		<!-- Available Tools Section -->
		<Section label="Available Tools">
			{#snippet action()}
				<Button
					size="xs"
					color="light"
					on:click={() => (refreshCount += 1)}
					startIcon={{ icon: RefreshCw }}
					disabled={tools.status === 'loading'}
				>
					{tools.status === 'loading' ? 'Loading...' : 'Refresh Tools'}
				</Button>
			{/snippet}
			<div class="w-full flex flex-col gap-2">
				{#if tools.error}
					<div class="text-xs text-red-600 p-2 border border-red-300 rounded bg-red-50">
						{tools.error?.body?.message ||
							tools.error?.message ||
							'Failed to load tools from MCP server'}
					</div>
				{/if}
				<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
					{#if tools.status === 'loading'}
						<div class="text-xs text-secondary italic">Loading tools...</div>
					{:else if (tools.value ?? []).length === 0}
						<div class="text-xs text-secondary italic">
							{tools.error
								? 'Failed to load tools. Please check the resource path and try again.'
								: 'No tools loaded yet. Click "Refresh Tools" to fetch tools from the MCP server.'}
						</div>
					{:else}
						<div class="flex flex-col gap-1">
							{#each tools.value ?? [] as tool}
								<div class="text-xs">
									<span class="font-semibold">{tool.name}</span>
									{#if tool.description}
										<span class="text-secondary">— {tool.description}</span>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		</Section>

		<!-- Tool Filtering Section -->
		{#if tool.value.include_tools && tool.value.exclude_tools}
			<Section label="Tool Filtering">
				<div class="w-full flex flex-col gap-3">
					<div class="flex flex-col gap-2">
						<Label label="Only include specified tools">
							<MultiSelect
								bind:value={tool.value.include_tools}
								items={toolOptions}
								placeholder="Choose tools to include..."
							/>
						</Label>
					</div>
					<div class="flex flex-col gap-2">
						<Label label="Exclude specified tools">
							<MultiSelect
								bind:value={tool.value.exclude_tools}
								items={toolOptions}
								placeholder="Choose tools to exclude..."
							/>
						</Label>
					</div>
				</div>
			</Section>
		{/if}
	{/if}
</div>

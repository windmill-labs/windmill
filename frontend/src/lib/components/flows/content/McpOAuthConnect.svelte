<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import {
		McpOauthService,
		OauthService,
		ResourceService,
		VariableService,
		type DiscoverMcpOauthResponse
	} from '$lib/gen'
	import { Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import { sendUserToast } from '$lib/toast'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { onDestroy } from 'svelte'

	interface Props {
		onConnected: (resourcePath: string, resourceName: string) => void
		onCancel: () => void
	}

	let { onConnected, onCancel }: Props = $props()

	let serverUrl = $state('')
	let discoveryResult = $state<DiscoverMcpOauthResponse | null>(null)
	let selectedScopes = $state<string[]>([])
	let resourceName = $state('')
	let resourcePath = $state('')
	let pathError = $state('')
	let status = $state<'idle' | 'discovering' | 'discovered' | 'connecting'>('idle')
	let error = $state<string | null>(null)

	async function discoverOAuth() {
		status = 'discovering'
		error = null
		try {
			discoveryResult = await McpOauthService.discoverMcpOauth({
				requestBody: { mcp_server_url: serverUrl }
			})
			selectedScopes = discoveryResult?.scopes_supported ?? []
			try {
				const urlObj = new URL(serverUrl)
				resourceName = urlObj.hostname.replace(/\./g, '_')
			} catch {
				resourceName = 'mcp_server'
			}
			status = 'discovered'
		} catch (e: any) {
			error = e.body?.message || e.message || 'Failed to discover OAuth settings'
			status = 'idle'
		}
	}

	function startOAuth() {
		const url = new URL(`/api/mcp/oauth/start`, window.location.origin)
		url.searchParams.set('mcp_server_url', serverUrl)
		url.searchParams.set('scopes', selectedScopes.join(','))

		const popup = window.open(url.toString(), '_blank', 'popup=true')
		if (!popup) {
			error = 'Popup blocked. Please allow popups for this site.'
			return
		}

		window.addEventListener('message', handleOAuthMessage)
		window.addEventListener('storage', handleStorageEvent)
		status = 'connecting'
	}

	function handleOAuthMessage(event: MessageEvent) {
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) return

		if (event.data.type === 'MCP_CONNECTED') {
			cleanup()
			createMcpResource(event.data)
		} else if (event.data.type === 'MCP_ERROR') {
			cleanup()
			error = event.data.error
			status = 'discovered'
		}
	}

	function handleStorageEvent(event: StorageEvent) {
		if (event.key === 'mcp-oauth-callback') {
			cleanup()
			try {
				const data = JSON.parse(event.newValue || '{}')
				localStorage.removeItem('mcp-oauth-callback')
				if (data.type === 'MCP_CONNECTED') {
					createMcpResource(data)
				}
			} catch {
				// Silently ignore invalid callback data
			}
		}
	}

	function cleanup() {
		window.removeEventListener('message', handleOAuthMessage)
		window.removeEventListener('storage', handleStorageEvent)
	}

	async function createMcpResource(data: {
		access_token: string
		refresh_token?: string
		expires_in?: number
		mcp_server_url: string
	}) {
		try {
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

			sendUserToast('Connected to MCP server')
			onConnected(resourcePath, resourceName)
		} catch (e: any) {
			error = e.body?.message || e.message || 'Failed to create resource'
			status = 'discovered'
		}
	}

	onDestroy(cleanup)
</script>

<div class="border rounded p-4 bg-surface-secondary flex flex-col gap-4">
	<div class="flex justify-between items-center">
		<span class="font-semibold text-sm">Connect MCP Server with OAuth</span>
		<Button size="xs" color="light" on:click={onCancel}>Cancel</Button>
	</div>

	<Label label="MCP Server URL">
		<input
			type="url"
			bind:value={serverUrl}
			placeholder="https://mcp.example.com"
			class="text-sm w-full"
			disabled={status === 'connecting'}
		/>
	</Label>

	{#if status === 'idle'}
		<Button size="sm" on:click={discoverOAuth} disabled={!serverUrl}>
			Discover OAuth Settings
		</Button>
	{:else if status === 'discovering'}
		<div class="text-sm text-secondary">Discovering OAuth settings...</div>
	{:else if status === 'discovered' && discoveryResult}
		<div class="text-xs text-green-600 dark:text-green-400">
			&#10003; OAuth supported
			{#if discoveryResult.supports_dynamic_registration}
				(Dynamic Client Registration available)
			{/if}
		</div>

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

		<Label label="Resource Name">
			<input
				type="text"
				bind:value={resourceName}
				placeholder="my-mcp-server"
				class="text-sm w-full"
			/>
		</Label>

		<Path
			bind:path={resourcePath}
			bind:error={pathError}
			initialPath=""
			namePlaceholder={resourceName}
			kind="resource"
		/>

		<Button size="sm" on:click={startOAuth} disabled={!resourcePath || pathError !== ''}>
			Connect with OAuth
		</Button>
	{:else if status === 'connecting'}
		<div class="text-sm text-secondary">Complete authentication in popup window...</div>
	{/if}

	{#if error}
		<div class="text-xs text-red-600 dark:text-red-400">{error}</div>
	{/if}
</div>

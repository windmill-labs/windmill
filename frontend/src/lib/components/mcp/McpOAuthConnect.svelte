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
	import { Check } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { onDestroy, onMount } from 'svelte'

	interface Props {
		onConnected: (path: string, resourceName: string) => void
		/** The server to sign in to. Discovery runs against it on mount. */
		server: { name: string; url: string }
		/** Where the resource and its token variable land. */
		path: string
		workspace?: string
	}

	let { onConnected, server, path, workspace }: Props = $props()

	let opWs = $derived(workspace ?? $workspaceStore)

	let serverUrl = $derived(server.url)
	let resourceName = $derived(server.name.toLowerCase().replace(/[^a-z0-9]/g, '_'))
	let discoveryResult = $state<DiscoverMcpOauthResponse | null>(null)
	let selectedScopes = $state<string[]>([])
	let status = $state<'idle' | 'discovering' | 'discovered' | 'connecting'>('idle')
	let error = $state<string | null>(null)
	let noOAuth = $state(false)

	async function discoverOAuth() {
		status = 'discovering'
		error = null
		try {
			discoveryResult = await McpOauthService.discoverMcpOauth({
				requestBody: { mcp_server_url: serverUrl }
			})
			selectedScopes = discoveryResult?.scopes_supported ?? []
			noOAuth = false
			status = 'discovered'
		} catch (e) {
			console.error('Error discovering OAuth settings', e)
			noOAuth = true
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
			} catch (e) {
				console.error('Error parsing MCP OAuth callback', e)
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
					workspace: opWs!,
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
				workspace: opWs!,
				requestBody: {
					path: path,
					value: data.access_token,
					is_secret: true,
					is_oauth: true,
					account: accountId,
					description: `MCP OAuth token for ${data.mcp_server_url}`
				}
			})

			await ResourceService.createResource({
				workspace: opWs!,
				requestBody: {
					resource_type: 'mcp',
					path: path,
					value: {
						name: resourceName,
						url: data.mcp_server_url,
						token: `$var:${path}`
					},
					description: `MCP server connected via OAuth`
				}
			})

			sendUserToast('Connected to MCP server')
			onConnected(path, resourceName)
		} catch (e: any) {
			error = e.body?.message || e.message || 'Failed to create resource'
			status = 'discovered'
		}
	}

	onMount(discoverOAuth)

	onDestroy(cleanup)
</script>

<div class="flex flex-col gap-4">
	{#if status === 'idle'}
		{#if noOAuth}
			<div class="text-2xs text-secondary">
				{server.name} did not advertise OAuth support, so connect with a token below.
			</div>
		{/if}
		<Button
			unifiedSize="2xs"
			variant="subtle"
			wrapperClasses="self-start"
			onClick={discoverOAuth}
			disabled={!serverUrl}
		>
			{noOAuth ? 'Check again' : 'Check for OAuth support'}
		</Button>
	{:else if status === 'discovering'}
		<div class="text-xs text-secondary">Checking what {server.name} supports...</div>
	{:else if status === 'discovered' && discoveryResult}
		<div class="text-xs text-green-600 dark:text-green-400 flex items-center gap-1">
			<Check size={14} />
			OAuth supported
			{#if discoveryResult.supports_dynamic_registration}
				(dynamic client registration available)
			{/if}
		</div>

		{#if discoveryResult.scopes_supported && discoveryResult.scopes_supported.length > 0}
			<Label label="Scopes">
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

		<Button
			unifiedSize="sm"
			variant="accent"
			wrapperClasses="self-start"
			onClick={startOAuth}
			disabled={!path}
		>
			Sign in with {server.name}
		</Button>
	{:else if status === 'connecting'}
		<div class="text-xs text-secondary">Complete authentication in the popup window...</div>
	{/if}

	{#if error}
		<div class="text-xs text-red-600 dark:text-red-400">{error}</div>
	{/if}
</div>

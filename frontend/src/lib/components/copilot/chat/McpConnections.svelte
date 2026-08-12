<script lang="ts">
	import { Button, Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import AppConnectDrawer from '$lib/components/AppConnectDrawer.svelte'
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Plug, Trash2 } from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	const GITHUB_MCP_URL = 'https://api.githubcopilot.com/mcp/'

	const aiChatManager = getAiChatManager()

	let drawer: Drawer | undefined = $state(undefined)
	let appConnectDrawer: AppConnectDrawer | undefined = $state(undefined)
	let servers = $state<{ path: string; description?: string }[]>([])
	let loading = $state(false)

	async function loadServers() {
		if (!$workspaceStore) return
		loading = true
		try {
			const resources = await ResourceService.listResource({
				workspace: $workspaceStore,
				resourceType: 'mcp',
				perPage: 100
			})
			servers = resources.map((r) => ({ path: r.path, description: r.description }))
		} finally {
			loading = false
		}
	}

	export async function open() {
		drawer?.openDrawer()
		await loadServers()
	}

	// An mcp resource's `token` is always read as a variable path, so it must be a
	// `$var:` reference. The OAuth connect and the "Secret" token entry both leave
	// one on the github resource, and reusing it verbatim also keeps the OAuth
	// account link (and so the refresh) intact. An inlined token leaves a literal
	// instead, which has to be moved into a variable of its own first.
	async function tokenRefFor(githubPath: string): Promise<string> {
		const resource = await ResourceService.getResource({
			workspace: $workspaceStore!,
			path: githubPath
		})
		const token = (resource.value as { token?: unknown } | undefined)?.token
		if (typeof token !== 'string' || token === '') {
			throw new Error(`No token found on the connected github resource ${githubPath}`)
		}
		if (token.startsWith('$var:')) {
			return token
		}
		const varPath = `${githubPath}_mcp_token`
		await VariableService.createVariable({
			workspace: $workspaceStore!,
			requestBody: {
				path: varPath,
				value: token,
				is_secret: true,
				description: `GitHub token for the ${githubPath}_mcp MCP server`
			}
		})
		return `$var:${varPath}`
	}

	async function onGithubConnected(githubPath: string) {
		const path = `${githubPath}_mcp`
		try {
			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type: 'mcp',
					path,
					value: {
						name: 'github',
						url: GITHUB_MCP_URL,
						token: await tokenRefFor(githubPath)
					},
					description: 'GitHub MCP server, called with your own GitHub credentials'
				}
			})
			sendUserToast(`Connected GitHub to the chat as ${path}`)
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to connect GitHub: ${e.body ?? e.message}`, true)
		}
	}

	async function disconnect(path: string) {
		try {
			await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
			sendUserToast(`Disconnected ${path}`)
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to disconnect ${path}: ${e.body ?? e.message}`, true)
		}
	}

	async function refresh() {
		await loadServers()
		// Re-register the chat's MCP tools so a connection made here is usable in
		// the next message without a reload.
		await aiChatManager.refreshMcpServers()
	}
</script>

<Button
	unifiedSize="2xs"
	variant="subtle"
	startIcon={{ icon: Plug }}
	btnClasses="text-secondary font-normal"
	title="MCP connections"
	onClick={open}
>
	Connections
</Button>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="MCP connections"
		on:close={() => drawer?.closeDrawer()}
		tooltip="Connect an external MCP server to this chat. The chat calls its tools with your own credentials, so it can only reach what you can."
	>
		<div class="flex flex-col gap-4">
			<div>
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Plug }}
					on:click={() => appConnectDrawer?.open('github')}
				>
					Connect GitHub
				</Button>
			</div>

			{#if loading}
				<div class="flex justify-center p-4"><Loader2 class="animate-spin" /></div>
			{:else if servers.length === 0}
				<div class="text-xs text-tertiary">
					No MCP server connected. Connect GitHub above, or create a resource of type
					<code>mcp</code> from the Resources page to connect any other MCP server.
				</div>
			{:else}
				<div class="flex flex-col divide-y border rounded-md">
					{#each servers as server (server.path)}
						<div class="flex items-center gap-2 px-3 py-2">
							<div class="min-w-0 grow">
								<div class="text-sm truncate">{server.path}</div>
								{#if server.description}
									<div class="text-2xs text-tertiary truncate">{server.description}</div>
								{/if}
							</div>
							<Button
								unifiedSize="2xs"
								variant="subtle"
								startIcon={{ icon: Trash2 }}
								iconOnly
								title="Disconnect"
								on:click={() => disconnect(server.path)}
							/>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<AppConnectDrawer bind:this={appConnectDrawer} on:refresh={(e) => onGithubConnected(e.detail)} />

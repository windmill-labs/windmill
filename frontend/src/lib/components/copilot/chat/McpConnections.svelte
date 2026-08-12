<script lang="ts">
	import { Button, Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import McpConnect from '$lib/components/mcp/McpConnect.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { isMcpEnabled, setMcpEnabled } from '$lib/components/mcp/enabledServers'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Plug, Plus, Trash2 } from 'lucide-svelte'
	import type { Item } from '$lib/utils'
	import { getAiChatManager } from './aiChatManagerContext'
	import { clearMcpToolsCache } from './global/mcpTools'

	const aiChatManager = getAiChatManager()
	// A session chat operates on its own (possibly forked) workspace without
	// switching `workspaceStore`, and that is the workspace the chat reads the
	// enabled set under. Key everything here the same way or a toggle lands under
	// a key nothing reads.
	//
	// `operatingWorkspace` is a plain getter over untracked state, so the store is
	// read unconditionally rather than behind `??`: short-circuiting it would leave
	// this derived with no dependency at all, frozen on the workspace it first saw.
	let ws = $derived.by(() => {
		const active = $workspaceStore
		return aiChatManager.operatingWorkspace ?? active!
	})

	let drawer: Drawer | undefined = $state(undefined)
	let showConnect = $state(false)
	let servers = $state<{ path: string; description?: string; enabled: boolean }[]>([])
	let loading = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let pendingDisconnect = $state<string | undefined>(undefined)

	async function loadServers() {
		if (!ws) return
		loading = true
		loadError = undefined
		try {
			const resources = await ResourceService.listResource({
				workspace: ws,
				resourceType: 'mcp',
				perPage: 100
			})
			servers = resources.map((r) => ({
				path: r.path,
				description: r.description,
				enabled: isMcpEnabled(ws, r.path)
			}))
		} catch (e) {
			// Without this the drawer would render the empty state, which reads as
			// "you have no connections" rather than "we could not load them".
			loadError = e.body ?? e.message
		} finally {
			loading = false
		}
	}

	export async function open() {
		drawer?.openDrawer()
		await loadServers()
	}

	/** Rows for the chat's "+" menu: one per connected server, checked when it is
	 * on, then the way to add another. Loaded on open so the checks are current. */
	export async function menuItems(): Promise<Item[]> {
		await loadServers()
		return [
			...servers.map((server) => ({
				displayName: server.path,
				toggle: server.enabled,
				action: () => toggle(server.path, !server.enabled)
			})),
			{
				displayName: 'Connect a server',
				icon: Plus,
				separatorTop: servers.length > 0,
				action: () => void open()
			}
		]
	}

	async function toggle(path: string, enabled: boolean) {
		// Local preference only: nothing to re-read from the API, and the cached
		// tool lists stay valid because the servers are unchanged.
		setMcpEnabled(ws, path, enabled)
		const server = servers.find((s) => s.path === path)
		if (server) server.enabled = enabled
		await aiChatManager.refreshMcpServers()
	}

	// Deleting a resource also deletes every variable its value references, and an
	// mcp resource's token is usually the credential of the resource it was created
	// from (the github one). Drop the reference before deleting so disconnecting
	// here can never destroy a credential something else still uses; the variable
	// is left for the user to remove from the Variables page.
	async function disconnect(path: string) {
		try {
			const resource = await ResourceService.getResource({
				workspace: ws,
				path
			})
			const { token: _token, ...withoutToken } = (resource.value ?? {}) as Record<string, unknown>
			await ResourceService.updateResource({
				workspace: ws,
				path,
				requestBody: { value: withoutToken }
			})
			await ResourceService.deleteResource({ workspace: ws, path })
			// A later resource at this path is a different server; it must be turned
			// on deliberately rather than inherit this one's enablement.
			setMcpEnabled(ws, path, false)
			sendUserToast(`Disconnected ${path}. Its token variable was kept.`)
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to disconnect ${path}: ${e.body ?? e.message}`, true)
		} finally {
			pendingDisconnect = undefined
		}
	}

	async function refresh() {
		// A path can be reconnected to a different server, so the cached tool list
		// (and the readOnlyHint the confirmation gate reads) must not survive.
		clearMcpToolsCache()
		await loadServers()
		// Re-register the chat's MCP tools so a connection made here is usable in
		// the next message without a reload.
		await aiChatManager.refreshMcpServers()
	}
</script>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="MCP connections"
		on:close={() => drawer?.closeDrawer()}
		tooltip="Connect an external MCP server to this chat. The chat calls its tools with your own credentials, so it can only reach what you can."
	>
		<div class="flex flex-col gap-4">
			{#if showConnect}
				<McpConnect
					workspace={ws}
					onConnected={async (path) => {
						// Connecting one is the act of choosing it.
						if (!setMcpEnabled(ws, path, true)) {
							sendUserToast(`Connected ${path}, but could not turn it on. Toggle it here.`, true)
						}
						showConnect = false
						await refresh()
					}}
					onCancel={() => (showConnect = false)}
				/>
			{:else}
				<div class="flex">
					<Button
						variant="accent"
						unifiedSize="sm"
						startIcon={{ icon: Plug }}
						onClick={() => (showConnect = true)}
					>
						Connect a server
					</Button>
				</div>
			{/if}

			{#if loading}
				<div class="flex justify-center p-4"><Loader2 class="animate-spin" /></div>
			{:else if loadError}
				<div class="text-xs text-red-600 dark:text-red-400">
					Failed to load MCP connections: {loadError}
				</div>
			{:else if servers.length === 0}
				<div class="text-2xs text-tertiary">No MCP server connected yet.</div>
			{:else}
				<div class="flex flex-col divide-y border rounded-md">
					{#each servers as server (server.path)}
						<div class="flex items-center gap-2 px-3 py-2">
							<Toggle
								size="xs"
								checked={server.enabled}
								on:change={async (e) => await toggle(server.path, e.detail)}
							/>
							<div class="min-w-0 grow">
								<div class="text-xs font-mono text-emphasis truncate">{server.path}</div>
								{#if server.description}
									<div class="text-2xs text-secondary truncate">{server.description}</div>
								{/if}
							</div>
							{#if pendingDisconnect === server.path}
								<span class="text-2xs text-secondary shrink-0">Disconnect? Its token is kept.</span>
								<Button
									unifiedSize="2xs"
									variant="default"
									onClick={() => (pendingDisconnect = undefined)}
								>
									Cancel
								</Button>
								<Button
									unifiedSize="2xs"
									variant="accent"
									destructive
									onClick={() => disconnect(server.path)}
								>
									Disconnect
								</Button>
							{:else}
								<Button
									unifiedSize="2xs"
									variant="subtle"
									startIcon={{ icon: Trash2 }}
									iconOnly
									title="Disconnect"
									onClick={() => (pendingDisconnect = server.path)}
								/>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>

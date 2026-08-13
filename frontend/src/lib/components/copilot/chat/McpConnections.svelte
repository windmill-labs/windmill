<script lang="ts">
	import { Button, Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import McpConnect from '$lib/components/mcp/McpConnect.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { isMcpEnabled, setMcpEnabled } from '$lib/components/mcp/enabledServers'
	import { loadProviderIcon } from '$lib/components/mcp/providerIcon'
	import {
		cachedProviderKey,
		forgetProviderKey,
		rememberProviderKey
	} from '$lib/components/mcp/iconCache'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { Component } from 'svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { List, Loader2, Plug, Plus, Trash2 } from 'lucide-svelte'
	import type { Item } from '$lib/utils'
	import { untrack } from 'svelte'
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
	// Connecting is what the drawer is for, so the card is always up; remounting it
	// after a connection is what clears the fields for the next one.
	let connectSeq = $state(0)
	let servers = $state<
		{
			path: string
			description?: string
			editedAt?: string
			enabled: boolean
			icon?: Component<any>
		}[]
	>([])
	let loading = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let pendingDisconnect = $state<string | undefined>(undefined)

	// Rows describe one workspace. A switch while the drawer is open must not leave
	// A's rows on screen while the actions below target B: same path, different
	// server, and disconnect would delete the wrong one.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		untrack(() => {
			servers = []
			void loadServers(target)
		})
	})

	async function loadServers(target = ws) {
		if (!target) return
		const seq = ++loadSeq
		loading = true
		loadError = undefined
		try {
			const resources = await ResourceService.listResource({
				workspace: target,
				resourceType: 'mcp',
				perPage: 100
			})
			if (seq !== loadSeq) return
			servers = resources.map((r) => ({
				path: r.path,
				description: r.description,
				editedAt: r.edited_at,
				enabled: isMcpEnabled(target, r.path)
			}))
			void loadIcons(target, seq)
		} catch (e) {
			if (seq !== loadSeq) return
			// Without this the drawer would render the empty state, which reads as
			// "you have no connections" rather than "we could not load them".
			loadError = e.body ?? e.message
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	export async function open() {
		drawer?.openDrawer()
		await loadServers()
	}

	// A menu is a shortcut, not a directory: past this many the list stops being
	// scannable, so the rest are reached through the drawer rather than dropped.
	const MAX_MENU_SERVERS = 8

	/** Rows for the chat's "+" menu: one per connected server, checked when it is
	 * on, then the way to add another. Loaded on open so the checks are current. */
	export async function menuItems(closeMenu?: () => void): Promise<Item[]> {
		// The menu opens on what is already known and refreshes behind it: waiting on
		// a round trip would stall the whole `+` menu, attachments included.
		if (servers.length === 0) {
			await loadServers()
		} else {
			void loadServers()
		}
		// Enabled first: those are the ones a quick visit is most likely about.
		const ordered = [...servers].sort(
			(a, b) => Number(b.enabled) - Number(a.enabled) || a.path.localeCompare(b.path)
		)
		const shown = ordered.slice(0, MAX_MENU_SERVERS)
		return [
			...shown.map(({ path }) => ({
				displayName: path,
				// Getters, not snapshots: the menu stays open across a click, and it has
				// to read through the live list rather than the row captured here, since
				// a reload replaces every row object and a getter bound to the old one
				// would go on reporting the state it was built with.
				get icon() {
					// Plug where the provider is unknown, so one nameless server does not
					// pull its label out of line with the rest.
					return row(path)?.icon ?? Plug
				},
				get toggle() {
					return row(path)?.enabled ?? false
				},
				action: () => toggle(path, !row(path)?.enabled)
			})),
			...(ordered.length > shown.length
				? [
						{
							displayName: `Show all ${ordered.length}`,
							icon: List,
							action: () => {
								closeMenu?.()
								void open()
							}
						}
					]
				: []),
			{
				displayName: 'Connect a server',
				icon: Plus,
				separatorTop: servers.length > 0,
				action: () => {
					closeMenu?.()
					void open()
				}
			}
		]
	}

	function row(path: string) {
		return servers.find((s) => s.path === path)
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
			forgetProviderKey(ws, path)
			sendUserToast(`Disconnected ${path}. Its token variable was kept.`)
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to disconnect ${path}: ${e.body ?? e.message}`, true)
		} finally {
			pendingDisconnect = undefined
		}
	}

	// A row whose provider is already cached paints from the cache; the rest cost
	// one read each, and a long list stops asking rather than firing a request
	// storm at a screen nobody is reading that far down.
	const MAX_ICON_LOOKUPS = 20
	async function loadIcons(target: string, seq: number) {
		let lookups = 0
		await Promise.all(
			servers.map(async (server) => {
				let key = cachedProviderKey(target, server.path, server.editedAt)
				if (key === undefined) {
					if (lookups >= MAX_ICON_LOOKUPS) return
					lookups++
					try {
						const resource = await ResourceService.getResource({
							workspace: target,
							path: server.path
						})
						key = rememberProviderKey(
							target,
							server.path,
							(resource.value as { url?: unknown } | undefined)?.url,
							server.editedAt
						)
					} catch {
						return
					}
				}
				const icon = await loadProviderIcon(key)
				if (seq !== loadSeq) return
				server.icon = icon
			})
		)
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
			{#key connectSeq}
				<McpConnect
					workspace={ws}
					onConnected={async (path) => {
						// Connecting one is the act of choosing it.
						if (!setMcpEnabled(ws, path, true)) {
							sendUserToast(`Connected ${path}, but could not turn it on. Toggle it here.`, true)
						}
						connectSeq++
						await refresh()
					}}
				/>
			{/key}

			{#if loading}
				<div class="flex justify-center p-4"><Loader2 class="animate-spin" /></div>
			{:else if loadError}
				<div class="text-xs text-red-600 dark:text-red-400">
					Failed to load MCP connections: {loadError}
				</div>
			{:else if servers.length === 0}
				<div class="text-xs text-secondary">No MCP server connected yet.</div>
			{:else}
				<div class="flex flex-col divide-y border rounded-md bg-surface-tertiary">
					{#each servers as server (server.path)}
						<div class="flex items-center gap-3 px-4 py-3">
							{#if server.icon}
								{@const Icon = server.icon}
								<Icon width="16px" height="16px" class="shrink-0" />
							{:else}
								<Plug size={16} class="shrink-0 text-tertiary" />
							{/if}
							<div class="min-w-0 grow">
								<div class="text-xs font-semibold text-emphasis truncate">{server.path}</div>
								{#if server.description}
									<div class="text-xs text-secondary truncate">{server.description}</div>
								{/if}
							</div>
							<Toggle
								size="xs"
								checked={server.enabled}
								on:change={async (e) => await toggle(server.path, e.detail)}
							/>
							<Button
								unifiedSize="2xs"
								variant="subtle"
								startIcon={{ icon: Trash2 }}
								iconOnly
								title="Disconnect"
								onClick={() => (pendingDisconnect = server.path)}
							/>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<ConfirmationModal
			open={pendingDisconnect !== undefined}
			title="Disconnect MCP server"
			confirmationText="Disconnect"
			onConfirmed={() => pendingDisconnect && disconnect(pendingDisconnect)}
			onCanceled={() => (pendingDisconnect = undefined)}
		>
			<span class="text-xs text-primary">
				This deletes the resource at <span class="font-semibold">{pendingDisconnect}</span>, so the chat
				and any flow pointing at it lose the server. Its token variable is kept.
			</span>
		</ConfirmationModal>
	</DrawerContent>
</Drawer>

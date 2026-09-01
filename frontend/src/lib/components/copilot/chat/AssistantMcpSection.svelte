<!--
@component
The MCP connections section of the assistant settings modal: the form that connects
an external MCP server to this chat, and the servers already connected, each with the
switch that decides whether this chat carries its tools.
-->
<script lang="ts">
	import { Button, Section } from '$lib/components/common'
	import EmptyState from '$lib/components/common/emptyState/EmptyState.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
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
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Plug, Plus, Trash2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { clearMcpToolsCache } from './global/mcpTools'

	let {
		ws,
		count = $bindable(),
		blocksClose = $bindable()
	}: {
		/** The workspace the chat operates on, which is not always the one on screen. */
		ws: string
		/** Number of connected servers, for the sidebar badge. */
		count: number
		/** True while this section is in the middle of something the modal must not
		 * close under: its confirmation, or the connect popover. */
		blocksClose: boolean
	} = $props()

	const aiChatManager = getAiChatManager()

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
	// The connect form lives in a popover off the header action; the empty state's
	// button opens that same popover rather than a second copy of the form.
	let connectPopover: Popover | undefined = $state(undefined)
	let connectOpen = $state(false)

	$effect(() => {
		count = servers.length
	})
	$effect(() => {
		// The connect popover is portaled out of the modal, so a click in it is a
		// click outside the modal. `connectOpen` is already true by then — it flips
		// on the trigger click, which is inside — so the guard is up in time.
		blocksClose = pendingDisconnect !== undefined || connectOpen
	})

	// Rows describe one workspace. A switch while the section is open must not leave
	// A's rows on screen while the actions below target B: same path, different
	// server, and disconnect would delete the wrong one.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		untrack(() => {
			servers = []
			pendingDisconnect = undefined
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
			// Without this the section would render the empty state, which reads as
			// "you have no connections" rather than "we could not load them".
			loadError = e.body ?? e.message
		} finally {
			if (seq === loadSeq) loading = false
		}
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
		// Pinned for the whole sequence: a switch midway would strip and delete the
		// resource that happens to share this path in the workspace switched to.
		const target = ws
		try {
			const resource = await ResourceService.getResource({
				workspace: target,
				path
			})
			const { token: _token, ...withoutToken } = (resource.value ?? {}) as Record<string, unknown>
			await ResourceService.updateResource({
				workspace: target,
				path,
				requestBody: { value: withoutToken }
			})
			await ResourceService.deleteResource({ workspace: target, path })
			// A later resource at this path is a different server; it must be turned
			// on deliberately rather than inherit this one's enablement.
			setMcpEnabled(target, path, false)
			forgetProviderKey(target, path)
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

<Section
	label="MCP connections"
	description="External MCP servers this chat can call. Their tools run with your own credentials, so the chat can only reach what you can."
	class="flex flex-col gap-4"
>
	{#snippet action()}
		<Popover
			bind:this={connectPopover}
			bind:isOpen={connectOpen}
			placement="bottom-end"
			contentClasses="w-[34rem] max-w-[calc(100vw-2rem)] p-4"
			triggerAttrs={{ 'aria-label': 'Connect an MCP server' }}
		>
			{#snippet trigger()}
				<!-- Popover renders the real trigger button around this one and carries the
				     label; this is here for the design-system styling alone, so it is taken
				     out of the tab order and the accessibility tree. -->
				<Button
					nonCaptureEvent
					unifiedSize="sm"
					variant="accent"
					startIcon={{ icon: Plus }}
					tabindex={-1}
					aria-hidden="true"
				>
					Connect a server
				</Button>
			{/snippet}
			{#snippet content({ close })}
				<!-- Closing unmounts the form, which is what clears it for the next connection. -->
				<McpConnect
					bordered={false}
					workspace={ws}
					onConnected={async (connectedWs, path) => {
						// Connecting one is the act of choosing it, and it is keyed on where
						// it was created rather than on what is on screen now: a switch
						// during the popup would otherwise enable the path in a workspace
						// that has no such connection.
						if (!setMcpEnabled(connectedWs, path, true)) {
							sendUserToast(`Connected ${path}, but could not turn it on. Toggle it here.`, true)
						}
						close()
						await refresh()
					}}
				/>
			{/snippet}
		</Popover>
	{/snippet}

	{#if loading}
		<div class="flex justify-center p-4"><Loader2 class="animate-spin" /></div>
	{:else if loadError}
		<div class="text-xs text-red-600 dark:text-red-400">
			Failed to load MCP connections: {loadError}
		</div>
	{:else if servers.length === 0}
		<EmptyState
			icon={Plug}
			title="No MCP server connected"
			description="Connect an external MCP server and the chat can call its tools with your own credentials."
			action={{ label: 'Connect a server', icon: Plus, onClick: () => connectPopover?.open() }}
		/>
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
</Section>

<ConfirmationModal
	open={pendingDisconnect !== undefined}
	title="Disconnect MCP server"
	confirmationText="Disconnect"
	onConfirmed={() => {
		if (pendingDisconnect) void disconnect(pendingDisconnect)
	}}
	onCanceled={() => (pendingDisconnect = undefined)}
>
	<span class="text-xs text-primary">
		This deletes the resource at <span class="font-semibold">{pendingDisconnect}</span>, so the chat
		and any flow pointing at it lose the server. Its token variable is kept.
	</span>
</ConfirmationModal>

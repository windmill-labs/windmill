<!--
@component
The MCP connections section of the assistant settings modal: the form that connects
an external MCP server to this chat, and the servers already connected, each with the
switch that decides whether this chat carries its tools.
-->
<script lang="ts">
	import { Button, Section } from '$lib/components/common'
	import EmptyState from '$lib/components/common/emptyState/EmptyState.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import McpConnect from '$lib/components/mcp/McpConnect.svelte'
	import ResourceEditor from '$lib/components/ResourceEditor.svelte'
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
	import { ArrowLeft, Loader2, Plug, Plus, Trash2 } from 'lucide-svelte'
	import { draftValuesEqual } from '$lib/userDraft.svelte'
	import { untrack } from 'svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { clearMcpToolsCache } from './global/mcpTools'

	let {
		ws,
		active,
		count = $bindable(),
		blocksClose = $bindable()
	}: {
		/** The workspace the chat operates on, which is not always the one on screen. */
		ws: string
		/** Whether this is the panel on screen. Gates the connect page's build, which
		 * costs a read of the instance OAuth connects — not worth paying for on a modal
		 * opened for one of the other three sections. */
		active: boolean
		/** Number of connected servers, for the sidebar badge. */
		count: number
		/** True while this section is in the middle of something the modal must not
		 * close under: its confirmation, or the connect form holding a half-filled
		 * connection. */
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
	// The connect form is the second page of this panel, not a surface over it.
	let connectOpen = $state(false)
	// Bumped as the connect page is left, which is what clears the form for the next
	// connection: the page stays mounted once built, so nothing else would. Bumped on
	// the way out rather than on the way in so the rebuild happens while the page is
	// parked and invisible, leaving every visit an already-built form.
	let connectSeq = $state(0)

	// The row being edited, on the third page. `ResourceEditor` is the same editor the
	// resource drawer opens, so a connection is edited here the way it is edited
	// anywhere else — url, token, description, path.
	let editing = $state<{ path: string; enabled: boolean } | undefined>(undefined)
	// The path the editor holds, which is not `editing.path` once someone renames it.
	let editingPath = $state('')
	let canSaveEditing = $state(false)
	let resourceEditor: ResourceEditor | undefined = $state(undefined)

	// What the form holds, which is what Save is measured against — not the stored value
	// the editor loaded. The schema form fills in a value for every property the resource
	// type declares, so an `mcp` resource saved without `headers` is holding
	// `headers: null` by the time it is on screen, and measuring against the stored value
	// would call that an edit the moment a connection is opened. The trigger editors take
	// their baseline from their own form for the same reason.
	//
	// It follows the form until the form is first focused, rather than being taken at a
	// moment judged to be after the fields have filled themselves in: those writes land
	// over several flushes and a snapshot timed against them is a guess, while nothing
	// here can be edited without focus reaching the form first.
	let editingBaseline = $state<unknown>(undefined)
	let editingTouched = $state(false)
	$effect(() => {
		const editor: ResourceEditor | undefined = resourceEditor
		// Snapshotting is also the deep read that makes this run again on every field.
		const settled = editor === undefined ? undefined : $state.snapshot(editor.localDraftCurrent())
		untrack(() => {
			if (editingTouched || settled === undefined) return
			editingBaseline = settled
		})
	})
	let editingChanged = $derived.by(() => {
		const editor: ResourceEditor | undefined = resourceEditor
		return !draftValuesEqual(editor?.localDraftCurrent(), editingBaseline)
	})
	// Bumped per visit: `ResourceEditor` reads its path once, on mount, and this page
	// stays mounted, so without a remount the second server opened would be the first.
	let detailSeq = $state(0)

	let page = $derived(connectOpen ? 'connect' : editing ? 'detail' : 'list')

	$effect(() => {
		count = servers.length
	})
	$effect(() => {
		blocksClose = pendingDisconnect !== undefined || connectOpen || editing !== undefined
	})

	/** Escape steps back to the list rather than closing the whole modal: `blocksClose`
	 * stops the modal's own handler, so this is the only thing left to answer the key. */
	function onKeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || page === 'list' || pendingDisconnect !== undefined) return
		event.preventDefault()
		event.stopPropagation()
		if (connectOpen) closeConnect()
		else closeDetail()
	}

	function openConnect() {
		connectOpen = true
	}

	function closeConnect() {
		connectOpen = false
		connectSeq++
	}

	function openServer(server: { path: string; enabled: boolean }) {
		editing = { path: server.path, enabled: server.enabled }
		editingPath = server.path
		// Dropped with the editor it was taken from: the next one settles on its own
		// connection, and comparing against the previous one's would call every field
		// an edit.
		editingBaseline = undefined
		editingTouched = false
		detailSeq++
	}

	function closeDetail() {
		editing = undefined
	}

	async function saveEditing() {
		const server = editing
		if (!server) return
		await resourceEditor?.save()
		// Enablement is keyed by path, so a rename would leave the switch on the path
		// that no longer exists and the server itself off.
		if (editingPath && editingPath !== server.path) {
			setMcpEnabled(ws, server.path, false)
			forgetProviderKey(ws, server.path)
			setMcpEnabled(ws, editingPath, server.enabled)
		}
		closeDetail()
		await refresh()
	}

	// Rows describe one workspace. A switch while the section is open must not leave
	// A's rows on screen while the actions below target B: same path, different
	// server, and disconnect would delete the wrong one.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		untrack(() => {
			servers = []
			pendingDisconnect = undefined
			// Back to the list too: the editor holds one workspace's resource, and the
			// path it is on names a different server in the workspace switched to.
			editing = undefined
			connectOpen = false
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

<svelte:window onkeydown={onKeydown} />

<!-- The list, a connection and the connect form are levels of one panel, so moving
     between them slides rather than cuts — the same shape the Skills panel uses for
     its editor. Warmed once this panel is on screen so the connect form is built
     before the click rather than inside the transition; the detail page holds a
     `ResourceEditor` for one path and only builds once a row is opened. -->
<PagedContent
	warm={active}
	class="grow min-h-0"
	current={page}
	pages={[
		{ key: 'list', content: listPage },
		{ key: 'detail', content: detailPage },
		{ key: 'connect', content: connectPage }
	]}
/>

{#snippet listPage()}
	<div class="grow min-h-0 overflow-y-auto pr-2">
		<Section
			label="MCP connections"
			description="External MCP servers this chat can call. Their tools run with your own credentials, so the chat can only reach what you can."
			class="flex flex-col gap-4"
		>
			{#snippet action()}
				<Button unifiedSize="sm" variant="accent" startIcon={{ icon: Plus }} onClick={openConnect}>
					Connect a server
				</Button>
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
					action={{ label: 'Connect a server', icon: Plus, onClick: openConnect }}
				/>
			{:else}
				<!-- `overflow-hidden`: the rows light up on hover and a square corner would
				     otherwise poke out of the card's rounded one. -->
				<div class="flex flex-col divide-y border rounded-md bg-surface-tertiary overflow-hidden">
					{#each servers as server (server.path)}
						<div class="flex items-center gap-3 px-4 py-3 hover:bg-surface-hover transition-colors">
							<!-- The row opens the connection. Only the label is the button: the switch
							     and the disconnect beside it are controls of their own, and nesting them
							     inside one would be invalid and would fire both. -->
							<button
								type="button"
								class="flex items-center gap-3 grow min-w-0 text-left"
								onclick={() => openServer(server)}
							>
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
							</button>
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
	</div>
{/snippet}

{#snippet detailPage()}
	<div class="grow min-h-0 overflow-y-auto pr-2">
		<div class="flex">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: ArrowLeft }}
				btnClasses="text-secondary"
				onClick={closeDetail}
			>
				MCP connections
			</Button>
		</div>
		<!-- `headerClass` keeps a long path on one line: the Save button shares the header
		     row and would otherwise wrap the title under itself. -->
		<!-- Titled by the path the editor holds rather than the row's, so a rename is
		     visible as it is typed and the header does not empty out while the page
		     slides away. -->
		<Section label={editingPath} wrapperClass="mt-1" headerClass="min-w-0 truncate pr-2 font-mono">
			{#snippet action()}
				<div class="flex justify-end shrink-0">
					<Button
						variant="accent"
						unifiedSize="sm"
						disabled={!canSaveEditing || !editingChanged}
						onClick={saveEditing}
					>
						Save
					</Button>
				</div>
			{/snippet}
			<!-- Freezes the baseline above: from the first focus on, what the form holds is
			     the user's doing rather than its own. -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div onfocusin={() => (editingTouched = true)}>
				{#key detailSeq}
					{#if detailSeq > 0}
						<!-- `editingPath` is seeded with the row's path before this remounts, which
					     is how the editor is told which resource to load: it reads its path once,
					     on mount, and reports a rename back through the same binding. -->
						<ResourceEditor
							bind:this={resourceEditor}
							bind:canSave={canSaveEditing}
							bind:path={editingPath}
							workspace={ws}
							resource_type="mcp"
						/>
					{/if}
				{/key}
			</div>
		</Section>
	</div>
{/snippet}

{#snippet connectPage()}
	<!-- The form takes the panel over rather than opening on top of it: a form stacked
	     on the settings modal leaves two surfaces arguing over which one a click or an
	     Escape belongs to. `McpConnect` carries its own heading, so there is no Section
	     around it. -->
	<div class="grow min-h-0 overflow-y-auto pr-2">
		<div class="flex">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: ArrowLeft }}
				btnClasses="text-secondary"
				onClick={closeConnect}
			>
				MCP connections
			</Button>
		</div>
		<div class="mt-1">
			{#key connectSeq}
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
						closeConnect()
						await refresh()
					}}
				/>
			{/key}
		</div>
	</div>
{/snippet}

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

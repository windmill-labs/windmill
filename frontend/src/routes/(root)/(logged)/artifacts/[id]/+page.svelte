<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { onMount } from 'svelte'
	import { resource } from 'runed'
	import {
		AlertTriangle,
		ArrowLeft,
		ClipboardList,
		Code,
		Code2,
		Eye,
		FileText,
		History,
		Link2Off,
		Lock,
		MessageSquare,
		RefreshCw,
		Share2,
		Trash2
	} from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { Button, EmptyState, Skeleton } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ArtifactBody from '$lib/components/copilot/chat/artifacts/ArtifactBody.svelte'
	import ArtifactExportButton from '$lib/components/copilot/chat/artifacts/ArtifactExportButton.svelte'
	import ArtifactShareDialog from '$lib/components/copilot/chat/artifacts/ArtifactShareDialog.svelte'
	import ArtifactVersionPicker from '$lib/components/copilot/chat/artifacts/ArtifactVersionPicker.svelte'
	import MermaidCodeBlock from '$lib/components/copilot/chat/artifacts/MermaidCodeBlock.svelte'
	import {
		deleteArtifactForever,
		markArtifactOpened,
		restoreArtifact,
		showVersion,
		trashArtifact
	} from '$lib/components/copilot/chat/artifacts/artifactLibrary'
	import {
		mergeArtifactRows,
		workspaceArtifacts
	} from '$lib/components/copilot/chat/artifacts/artifactRows'
	import {
		artifactPath,
		shareWorkspaceId,
		UNKNOWN_AUDIENCE,
		whoCanSee
	} from '$lib/components/copilot/chat/artifacts/artifactSharing'
	import {
		currentVersion,
		getArtifact,
		type ArtifactVersion,
		type PersistedArtifact
	} from '$lib/components/copilot/chat/artifacts/artifactsDB'
	import {
		ArtifactStaleError,
		SessionArtifactsStore
	} from '$lib/components/copilot/chat/artifacts/artifactsState.svelte'
	import { sessionPageHref, sessionState } from '$lib/components/sessions/sessionState.svelte'
	import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
	import { AiService, type AiArtifactWithContent } from '$lib/gen'
	import { userStore, usersWorkspaceStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { apiErrorMessage, type Item } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	const id = $derived(page.params.id ?? '')
	// The link names the artifact's workspace; the store only catches up once the layout applies it,
	// and tells a fork from its parent only once the user's workspaces are read.
	const workspacesRead = $derived($usersWorkspaceStore !== undefined)
	const workspace = $derived(
		page.url.searchParams.get('workspace') ??
			($workspaceStore && workspacesRead
				? shareWorkspaceId($workspaceStore, $userWorkspaces)
				: undefined)
	)
	const workspaceName = $derived(
		$userWorkspaces.find((w) => w.id === workspace)?.name ?? workspace ?? ''
	)

	// The page restores versions through the store the chat writes with, so a restored version
	// reaches the server the way an AI write does.
	const store = new SessionArtifactsStore(() => workspace)

	type Found = {
		state: 'found'
		/** This browser's copy, which only its owner has, with every version it kept. */
		local: PersistedArtifact | undefined
		/** The server's copy, with the version people with access see; absent until shared. */
		server: AiArtifactWithContent | undefined
		/** Why the server's copy could not be read, when this browser's copy is shown without it. */
		serverError?: string
	}
	type Loaded = Found | { state: 'gone' } | { state: 'error'; message: string }

	type ServerRead = { found?: AiArtifactWithContent; error?: string }

	async function load(workspace: string, id: string): Promise<Loaded> {
		const [stored, server] = await Promise.all([
			getArtifact(id).catch(() => undefined),
			AiService.getAiArtifact({ workspace, artifactId: id }).then(
				(found): ServerRead => ({ found }),
				(err): ServerRead => {
					const status = (err as { status?: number })?.status
					// A malformed id is a 400 from the path extractor, and means the same to the reader.
					return status === 404 || status === 400 ? {} : { error: apiErrorMessage(err) }
				}
			)
		])
		const held =
			stored && (stored.workspaceId === undefined || stored.workspaceId === workspace)
				? stored
				: undefined
		if (!held && server.error) return { state: 'error', message: server.error }
		if (!held && !server.found) return { state: 'gone' }
		// A version its owner wrote in another browser reaches this one through the server's copy.
		const local = held && server.found ? await store.catchUp(held, server.found) : held
		return { state: 'found', local, server: server.found, serverError: server.error }
	}

	let reloads = $state(0)
	/** A newer version that landed while the page was open. It waits for the reader to ask for
	 * it: the page never swaps the text they are reading. */
	let newer: Found | undefined = $state()

	const loaded = resource(
		() => ({ workspace, id, reloads }),
		async ({ workspace, id }) => {
			if (!workspace || !id) return undefined
			const result = await load(workspace, id)
			newer = undefined
			if (result.state === 'found') markArtifactOpened(id)
			return { id, result }
		}
	)

	// The header's switcher keeps this page mounted from one artifact to the next: a load, and a
	// version picked, count only for the artifact they were for.
	const current = $derived(loaded.current?.id === id ? loaded.current.result : undefined)
	const found = $derived(current?.state === 'found' ? current : undefined)
	const local = $derived(found?.local)
	const server = $derived(found?.server)
	const mine = $derived(local !== undefined || server?.is_owner === true)
	// This browser's copy decides, as on the artifacts page: a plan written again in its session is
	// live here while the server copy it was shared from stays in the trash.
	const deletedAt = $derived(
		local ? local.deletedAt : server?.deleted_at ? Date.parse(server.deleted_at) : undefined
	)
	/** The owner's copy while it is out of the trash: versions are read and written through it. */
	const liveLocal = $derived(local?.deletedAt === undefined ? local : undefined)
	/** The owner, or an admin, who changes who has access to any item on the server. */
	const canShare = $derived(
		deletedAt === undefined && (mine || (server !== undefined && !!$userStore?.is_admin))
	)

	/** An older version the owner picked, shown in place of the latest. */
	let picked: { id: string; version: ArtifactVersion } | undefined = $state()
	const snapshot = $derived(picked?.id === id ? picked.version : undefined)
	const text = $derived(snapshot ?? local ?? server)
	const kind = $derived(local?.kind ?? server?.kind ?? 'md')
	const role = $derived(local?.role ?? server?.role)
	const latest = $derived(liveLocal ? currentVersion(liveLocal) : undefined)
	const shared = $derived(
		server !== undefined && !server.deleted_at && Object.keys(server.extra_perms).length > 0
	)
	// Nobody else opens an artifact whose server copy is in the trash, whoever its access list names.
	const who = $derived(
		found?.serverError
			? UNKNOWN_AUDIENCE
			: whoCanSee(
					server && !server.deleted_at ? server.extra_perms : undefined,
					workspaceName,
					mine ? undefined : server?.owner
				)
	)

	const session = $derived.by(() => {
		const sessionId = local?.sessionId ?? server?.session_id
		return mine && sessionId ? sessionState.sessions.find((s) => s.id === sessionId) : undefined
	})

	// The header's switcher lists what the artifacts page does, outside Recently deleted.
	const siblings = resource(
		() => ({ workspace, ready: sessionState.hydrated && workspacesRead, reloads }),
		async ({ workspace, ready }) => {
			if (!workspace) return []
			const [mineHere, listed] = await Promise.all([
				workspaceArtifacts(workspace, ready).catch(() => []),
				AiService.listAiArtifacts({ workspace }).catch(() => [])
			])
			return mergeArtifactRows(mineHere, listed).sort((a, b) => b.updatedAt - a.updatedAt)
		}
	)
	const navItems = $derived.by(() => {
		const items = (siblings.current ?? []).map((r) => ({ id: r.id, name: r.name }))
		if (text && !items.some((a) => a.id === id)) items.unshift({ id, name: text.name })
		return items
	})

	type Banner =
		| { kind: 'trash'; at: number }
		| { kind: 'older'; version: number; latest: number }
		| { kind: 'updated'; version: number }
		| { kind: 'unreachable'; message: string }
		| { kind: 'behind'; shown: number; latest: number }
		| { kind: 'shared'; owner: string; left: boolean; updatedAt: string }

	/** One banner at a time, the first that applies. */
	const banner = $derived.by((): Banner | undefined => {
		if (!found) return undefined
		if (mine && deletedAt !== undefined) return { kind: 'trash', at: deletedAt }
		if (snapshot && latest !== undefined) {
			return { kind: 'older', version: snapshot.version, latest }
		}
		const landed = newer && shownVersion(newer)
		if (landed !== undefined) return { kind: 'updated', version: landed }
		if (found.serverError) return { kind: 'unreachable', message: found.serverError }
		if (server && shared && latest !== undefined && server.shown_version < latest) {
			return { kind: 'behind', shown: server.shown_version, latest }
		}
		if (!mine && server) {
			return {
				kind: 'shared',
				owner: server.owner,
				left: !server.owner_is_member,
				updatedAt: server.updated_at
			}
		}
		return undefined
	})

	/** The version a load shows when no older one is picked. */
	function shownVersion(l: Found): number | undefined {
		return l.local ? currentVersion(l.local) : l.server?.shown_version
	}

	async function checkForNewer() {
		const shown = loaded.current
		if (!workspace || shown?.id !== id || shown.result.state !== 'found') return
		const before = shown.result
		const fresh = await load(workspace, shown.id)
		if (fresh.state !== 'found' || loaded.current !== shown) return
		const version = shownVersion(fresh)
		if (version !== undefined && version !== shownVersion(before)) newer = fresh
	}

	// Nothing pushes new versions to this page: one written elsewhere is found on focus and once a
	// minute.
	onMount(() => {
		const check = () => void checkForNewer()
		window.addEventListener('focus', check)
		const timer = setInterval(check, 60_000)
		return () => {
			window.removeEventListener('focus', check)
			clearInterval(timer)
		}
	})

	function showNewer() {
		if (!newer) return
		picked = undefined
		loaded.mutate({ id, result: newer })
		newer = undefined
	}

	let reading = 0
	async function selectVersion(version: number | undefined) {
		const seq = ++reading
		const artifact = liveLocal
		if (version === undefined || !artifact) {
			picked = undefined
			return
		}
		try {
			const read = await store.getVersion(artifact.id, version)
			if (seq !== reading) return
			if (read) picked = { id: artifact.id, version: read }
			else sendUserToast(`Version ${version} is no longer in this browser`, true)
		} catch (err) {
			if (seq === reading) {
				sendUserToast(`Could not read version ${version}`, true, [], apiErrorMessage(err))
			}
		}
	}

	let busy = $state(false)
	async function run(action: () => Promise<void>, failure: string) {
		busy = true
		try {
			await action()
		} catch (err) {
			sendUserToast(failure, true, [], apiErrorMessage(err))
		} finally {
			busy = false
		}
	}

	/** Save the version being viewed again as the latest. The name stays the current one. */
	function restoreVersion() {
		const from = snapshot
		const artifact = liveLocal
		if (!from || !artifact) return
		return run(async () => {
			try {
				const saved = await store.update(artifact.id, {
					content: from.content,
					note: `Restored v${from.version}`
				})
				if (!saved) {
					sendUserToast('This artifact is no longer in this browser', true)
				} else if (currentVersion(saved) === currentVersion(artifact)) {
					sendUserToast(`v${from.version} is the same as the latest version`)
				} else {
					logFeatureUsage('ai_artifacts', 'restore', { key: 'version' })
					sendUserToast(`Restored v${from.version} as v${currentVersion(saved)}`)
				}
			} catch (err) {
				if (!(err instanceof ArtifactStaleError)) throw err
				if (err.savedAs !== undefined) {
					logFeatureUsage('ai_artifacts', 'restore', { key: 'version' })
					sendUserToast(
						`Restored v${from.version} as v${err.savedAs}, and another tab has since saved v${err.latest}`
					)
				} else {
					sendUserToast(
						`This artifact changed in another tab or browser (now v${err.latest}), so nothing was restored`,
						true
					)
				}
			}
			picked = undefined
			reloads++
		}, `Could not restore v${from.version}`)
	}

	function showLatestToEveryone() {
		const artifact = liveLocal
		if (!artifact || !server || !workspace) return
		return run(async () => {
			const result = await showVersion(artifact, workspace, server.latest_version, {
				pinned: false,
				version: currentVersion(artifact),
				name: artifact.name,
				content: artifact.content
			})
			if (result.outcome === 'saved') {
				logFeatureUsage('ai_artifacts', 'pin', { key: 'latest' })
			} else if (result.outcome === 'stale') {
				sendUserToast(
					`Version ${result.latest_version} was written in another browser, and is not in this one yet`,
					true
				)
			}
			reloads++
		}, 'Could not change the version people see')
	}

	function trash() {
		if (!workspace) return
		const ws = workspace
		return run(
			async () => {
				await trashArtifact(id, ws)
				picked = undefined
				reloads++
			},
			`Could not delete "${text?.name ?? 'the artifact'}"`
		)
	}

	function restoreFromTrash() {
		if (!workspace) return
		const ws = workspace
		return run(
			async () => {
				await restoreArtifact(id, ws)
				logFeatureUsage('ai_artifacts', 'restore', { key: 'trash' })
				reloads++
			},
			`Could not restore "${text?.name ?? 'the artifact'}"`
		)
	}

	let shareDialog: ArtifactShareDialog | undefined = $state()
	async function share() {
		if (!text) return
		try {
			await shareDialog?.share({ id, name: text.name, local: liveLocal, path: server?.path })
		} catch (err) {
			sendUserToast('Could not share the artifact', true, [], apiErrorMessage(err))
		}
	}

	let confirmingDelete = $state(false)
	/** An admin removing someone else's artifact from the server: the trash is its owner's. */
	function deleteForEveryone() {
		const ws = workspace
		const name = server?.name ?? 'the artifact'
		if (!ws) return
		return run(async () => {
			await deleteArtifactForever(id, ws)
			confirmingDelete = false
			sendUserToast(`Deleted "${name}"`)
			await goto('/artifacts')
		}, `Could not delete "${name}"`)
	}

	const menu: Item[] = $derived([
		mine
			? {
					displayName: 'Delete',
					icon: Trash2,
					type: 'delete',
					disabled: busy,
					action: () => void trash()
				}
			: {
					displayName: 'Delete for everyone',
					icon: Trash2,
					type: 'delete',
					disabled: busy,
					action: () => (confirmingDelete = true)
				}
	])

	let showSource = $state(false)
	// Markdown is the one kind rendered; HTML shows its source, so no script of it ever runs.
	const source = $derived(kind !== 'md' || showSource)
	const Icon = $derived(role === 'plan' ? ClipboardList : kind === 'html' ? Code2 : FileText)

	const BAR = 'flex items-center gap-2 px-4 py-1.5 text-xs font-normal'
	const WARNING = 'bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300'
	const INFO = 'bg-surface-secondary text-secondary'

	function shortDate(ms: number): string {
		return new Date(ms).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
	}
</script>

{#snippet audience()}
	{#if who.audience === 'only_you'}
		<Lock size={12} class="shrink-0" />
	{/if}
	<span class="truncate">{who.label}</span>
	{#if server?.pinned && deletedAt === undefined}
		<span class="shrink-0 text-hint">· v{server.shown_version} pinned</span>
	{/if}
{/snippet}

<PageHeaderContent
	section={{
		label: 'Artifacts',
		navArtifacts: workspace
			? {
					items: navItems,
					currentId: id,
					href: (other) => artifactPath(workspace, other)
				}
			: undefined
	}}
/>

<div class="flex h-full min-h-0 flex-col">
	<div class="flex flex-wrap items-center gap-x-2 gap-y-1 border-b px-4 py-2">
		<Button variant="subtle" unifiedSize="sm" startIcon={{ icon: ArrowLeft }} href="/artifacts">
			Back to artifacts
		</Button>
		{#if found && text}
			<div class="flex min-w-0 flex-1 items-center gap-1.5">
				<Icon size={14} class="shrink-0 text-secondary" />
				<h1 class="truncate text-sm font-semibold text-emphasis" title={text.name}>{text.name}</h1>
				{#if canShare}
					<Button
						variant="subtle"
						unifiedSize="xs"
						wrapperClasses="min-w-0"
						btnClasses="min-w-0 gap-1 text-secondary hover:text-primary"
						title="Change who can see it"
						onclick={share}
					>
						{@render audience()}
					</Button>
				{:else}
					<span class="flex min-w-0 items-center gap-1 text-xs text-secondary">
						{@render audience()}
					</span>
				{/if}
				{#if liveLocal && currentVersion(liveLocal) > 1}
					<ArtifactVersionPicker
						artifact={liveLocal}
						{store}
						selected={snapshot?.version}
						onSelect={selectVersion}
					/>
				{/if}
			</div>
			<div class="flex shrink-0 items-center gap-2">
				{#if canShare}
					<Button variant="default" unifiedSize="sm" startIcon={{ icon: Share2 }} onclick={share}>
						Share
					</Button>
				{/if}
				{#if session}
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: MessageSquare }}
						href={withWorkspaceParam(
							sessionPageHref(session.id),
							session.workspace_id ?? session.pending_workspace_id
						)}
					>
						Open session
					</Button>
				{/if}
				<ArtifactExportButton name={text.name} {kind} content={text.content} />
				{#if kind === 'md'}
					<ToggleButtonGroup
						noWFull
						selected={showSource ? 'source' : 'preview'}
						onSelected={(v) => (showSource = v === 'source')}
					>
						{#snippet children({ item })}
							<ToggleButton
								{item}
								value="preview"
								icon={Eye}
								iconOnly
								tooltip="Preview"
								size="sm"
							/>
							<ToggleButton
								{item}
								value="source"
								icon={Code}
								iconOnly
								tooltip="View source"
								size="sm"
							/>
						{/snippet}
					</ToggleButtonGroup>
				{/if}
				{#if canShare}
					<DropdownV2 size="sm" items={menu} />
				{/if}
			</div>
		{/if}
	</div>

	{#if found && text}
		{#if banner?.kind === 'trash'}
			<div class="{BAR} {WARNING}">
				<Trash2 size={12} class="shrink-0" />
				<span class="truncate">
					Deleted on {shortDate(banner.at)}. It stays in Recently deleted for 30 days.
				</span>
				<div class="ml-auto shrink-0">
					<Button variant="default" unifiedSize="xs" disabled={busy} onclick={restoreFromTrash}>
						Restore
					</Button>
				</div>
			</div>
		{:else if banner?.kind === 'older'}
			<div class="{BAR} {WARNING}">
				<History size={12} class="shrink-0" />
				<span class="truncate">You're viewing v{banner.version} of {banner.latest}.</span>
				<div class="ml-auto flex shrink-0 gap-1">
					<Button variant="subtle" unifiedSize="xs" onclick={() => selectVersion(undefined)}>
						Back to latest
					</Button>
					<Button variant="default" unifiedSize="xs" disabled={busy} onclick={restoreVersion}>
						Restore
					</Button>
				</div>
			</div>
		{:else if banner?.kind === 'updated'}
			<div class="{BAR} {INFO}">
				<RefreshCw size={12} class="shrink-0" />
				<span class="truncate">Updated</span>
				<div class="ml-auto shrink-0">
					<Button variant="default" unifiedSize="xs" onclick={showNewer}>
						Show v{banner.version}
					</Button>
				</div>
			</div>
		{:else if banner?.kind === 'unreachable'}
			<div class="{BAR} {WARNING}">
				<AlertTriangle size={12} class="shrink-0" />
				<span class="truncate" title={banner.message}>
					Could not read this artifact from the server, so who can see it is unknown.
				</span>
				<div class="ml-auto shrink-0">
					<Button variant="default" unifiedSize="xs" onclick={() => reloads++}>Retry</Button>
				</div>
			</div>
		{:else if banner?.kind === 'behind'}
			<div class="{BAR} {WARNING}">
				<History size={12} class="shrink-0" />
				<span class="truncate">
					People with access see v{banner.shown}. You're viewing v{banner.latest}.
				</span>
				<div class="ml-auto shrink-0">
					<Button variant="default" unifiedSize="xs" disabled={busy} onclick={showLatestToEveryone}>
						Show v{banner.latest} to everyone
					</Button>
				</div>
			</div>
		{:else if banner?.kind === 'shared'}
			<div class="{BAR} {INFO}">
				<Share2 size={12} class="shrink-0" />
				<span class="truncate">
					Shared by {banner.owner}{banner.left ? ' (no longer in the workspace)' : ''} · updated
					<TimeAgo date={banner.updatedAt} compact /> ago
				</span>
			</div>
		{/if}

		<div class="min-h-0 flex-1 overflow-auto bg-surface-tertiary">
			<!-- Source fills the height; the rendered document grows with its text, which keeps the
			     body's sticky fade inside a box as tall as the scroll. -->
			<div class="mx-auto w-full max-w-4xl px-8 {source ? 'h-full' : 'min-h-full'}">
				<ArtifactBody content={text.content} {source} pre={MermaidCodeBlock} />
			</div>
		</div>
	{:else if current?.state === 'gone'}
		<EmptyState icon={Link2Off} title="This artifact isn't shared with you, or no longer exists" />
	{:else if current?.state === 'error'}
		<EmptyState
			icon={Link2Off}
			title="Could not load this artifact"
			description={current.message}
		/>
	{:else}
		<div class="mx-auto w-full max-w-4xl px-8 py-6">
			<Skeleton layout={[[3], 1, [30]]} />
		</div>
	{/if}
</div>

<ArtifactShareDialog bind:this={shareDialog} {workspace} onChange={() => reloads++} />
<ConfirmationModal
	open={confirmingDelete}
	title={`Delete "${server?.name ?? 'the artifact'}" for everyone?`}
	confirmationText="Delete"
	loading={busy}
	onCanceled={() => (confirmingDelete = false)}
	onConfirmed={deleteForEveryone}
>
	Nobody it is shared with can open it any more. Its owner keeps the copy in their own browser,
	unshared.
</ConfirmationModal>

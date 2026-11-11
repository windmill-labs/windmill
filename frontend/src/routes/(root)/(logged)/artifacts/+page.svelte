<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { onMount } from 'svelte'
	import { resource } from 'runed'
	import {
		AlertTriangle,
		ArrowLeft,
		ClipboardList,
		Code2,
		FileText,
		Files,
		Lock,
		MessageSquarePlus,
		RefreshCw,
		RotateCcw,
		Search,
		Trash2,
		Users
	} from 'lucide-svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { Alert, Button, EmptyState, Skeleton, Tab, Tabs } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Cell, DataTable, Row } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import ArtifactShareDialog from '$lib/components/copilot/chat/artifacts/ArtifactShareDialog.svelte'
	import {
		artifactOpens,
		deleteArtifactForever,
		restoreArtifact,
		sweepTrash
	} from '$lib/components/copilot/chat/artifacts/artifactLibrary'
	import {
		matchLine,
		mergeArtifactRows,
		workspaceArtifacts,
		type ArtifactRow
	} from '$lib/components/copilot/chat/artifacts/artifactRows'
	import {
		artifactPath,
		shareWorkspaceId,
		UNKNOWN_AUDIENCE,
		whoCanSee,
		type Audience
	} from '$lib/components/copilot/chat/artifacts/artifactSharing'
	import { sessionPageHref, sessionState } from '$lib/components/sessions/sessionState.svelte'
	import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
	import { openNewSession } from '$lib/components/sessions/sessionSwitch.svelte'
	import { AiService, type AiArtifactListEntry } from '$lib/gen'
	import { userStore, usersWorkspaceStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { apiErrorMessage, displayDate } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	type Sort = 'updated' | 'opened' | 'name'
	type KindFilter = 'md' | 'html' | 'plan'

	// The link carries its workspace; the store only catches up once the layout applies it. A fork
	// is told from its parent only through the user's workspaces, so nothing loads before they do.
	const current = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore)
	const workspacesRead = $derived($usersWorkspaceStore !== undefined)
	const home = $derived(
		current && workspacesRead ? shareWorkspaceId(current, $userWorkspaces) : undefined
	)
	const workspaceName = $derived($userWorkspaces.find((w) => w.id === home)?.name ?? home ?? '')
	const trashView = $derived(page.url.searchParams.get('view') === 'trash')
	/** The session the page was opened from, whose artifacts come first. */
	const fromSession = $derived(page.url.searchParams.get('session') ?? undefined)

	/** `all`, `mine` (Created by me) or `shared` (Shared with me). */
	let tab: string = $state('all')
	let search = $state('')
	let kind: KindFilter | undefined = $state()
	let audience: Audience | undefined = $state()
	let owner: string | undefined = $state()
	let sort: Sort = $state('updated')
	let reloads = $state(0)
	let shareDialog: ArtifactShareDialog | undefined = $state()

	const query = $derived(search.trim())
	const ME = '\u0000me'

	onMount(() => {
		logFeatureUsage('ai_artifacts', 'page_open')
		void sweepTrash()
	})

	let searched = false
	function noteSearch(q: string) {
		if (!q || searched) return
		searched = true
		logFeatureUsage('ai_artifacts', 'search')
	}

	// Undefined while `home` is, so the page shows it loading rather than empty.
	const local = resource(
		() => ({ home, ready: sessionState.hydrated && workspacesRead, reloads }),
		async ({ home, ready }) => (home ? await workspaceArtifacts(home, ready) : undefined)
	)

	// On `error`, the rows hold only this browser's artifacts, and who can see them is unknown.
	const server = resource(
		() => ({ home, trash: trashView, reloads }),
		async ({
			home,
			trash
		}): Promise<{ rows: AiArtifactListEntry[]; error?: string } | undefined> => {
			if (!home) return undefined
			try {
				return { rows: await AiService.listAiArtifacts({ workspace: home, includeDeleted: trash }) }
			} catch (err) {
				return { rows: [], error: apiErrorMessage(err) }
			}
		}
	)
	const listError = $derived(server.current?.error)

	// Content is on the server only for the artifacts this browser does not hold. Responses can
	// land out of order, so each carries the query it answers.
	const found = resource(
		() => ({ home, query, trash: trashView, reloads }),
		async ({ home, query, trash }) => {
			if (!home || !query) return undefined
			noteSearch(query)
			try {
				const rows = await AiService.listAiArtifacts({
					workspace: home,
					search: query,
					includeDeleted: trash
				})
				return { query, hits: new Map(rows.map((r) => [r.artifact_id, r.match_line])) }
			} catch {
				return { query, hits: new Map<string, string | undefined>() }
			}
		},
		{ debounce: 250 }
	)

	const opens = $derived.by(() => {
		void reloads
		return artifactOpens()
	})

	const loading = $derived(local.current === undefined || server.current === undefined)

	const rows = $derived.by(() => {
		let all = mergeArtifactRows(local.current ?? [], server.current?.rows ?? [], trashView)
		if (query) {
			const q = query.toLowerCase()
			const hits = found.current?.query === query ? found.current.hits : undefined
			all = all.flatMap((r) => {
				if (r.local) {
					const line = matchLine(r.local.content, query)
					return line !== undefined || r.name.toLowerCase().includes(q)
						? [{ ...r, matchLine: line }]
						: []
				}
				return hits?.has(r.id) ? [{ ...r, matchLine: hits.get(r.id) }] : []
			})
		}
		return all
	})

	const owners = $derived.by(() => {
		const names = rows.flatMap((r) => (!r.mine && r.owner ? [r.owner] : []))
		return names.filter((n, i) => names.indexOf(n) === i).sort((a, b) => a.localeCompare(b))
	})

	const shown = $derived.by(() => {
		const filtered = rows.filter(
			(r) =>
				(trashView || tab === 'all' || (tab === 'mine') === r.mine) &&
				(!kind ||
					(kind === 'plan'
						? r.role === 'plan'
						: r.kind === kind && (kind === 'html' || r.role !== 'plan'))) &&
				(!audience || audienceOf(r).audience === audience) &&
				(!owner || tab === 'mine' || (owner === ME ? r.mine : !r.mine && r.owner === owner))
		)
		const byName = (a: ArtifactRow, b: ArtifactRow) => a.name.localeCompare(b.name)
		const byUpdated = (a: ArtifactRow, b: ArtifactRow) => b.updatedAt - a.updatedAt
		return filtered.sort(
			sort === 'name'
				? byName
				: sort === 'opened'
					? (a, b) => (opens[b.id] ?? 0) - (opens[a.id] ?? 0) || byUpdated(a, b)
					: byUpdated
		)
	})

	const groups = $derived.by(() => {
		const here = fromSession ? shown.filter((r) => r.sessionId === fromSession) : []
		if (here.length === 0) return [{ label: undefined, rows: shown }]
		return [
			{ label: 'This session', rows: here },
			{ label: 'Other artifacts', rows: shown.filter((r) => r.sessionId !== fromSession) }
		].filter((g) => g.rows.length > 0)
	})

	const filtering = $derived(!!query || !!kind || !!audience || !!owner)

	function sessionOf(r: ArtifactRow) {
		if (!r.mine || !r.sessionId) return undefined
		return sessionState.sessions.find((s) => s.id === r.sessionId)
	}

	/** Only the browser that holds the artifact held its session, so only there is a missing
	 * session known to be deleted rather than kept in another browser. */
	function sessionDeleted(r: ArtifactRow): boolean {
		return !!r.local && !!r.sessionId && sessionState.hydrated && sessionOf(r) === undefined
	}

	function audienceOf(r: ArtifactRow): { audience: Audience; label: string } {
		return listError
			? UNKNOWN_AUDIENCE
			: whoCanSee(r.extraPerms, workspaceName, r.mine ? undefined : r.owner)
	}

	/** The owner, or an admin, who changes who has access to any item on the server. */
	function canShare(r: ArtifactRow): boolean {
		return r.mine || (!!r.path && !!$userStore?.is_admin)
	}

	function rowIcon(r: ArtifactRow) {
		return r.role === 'plan' ? ClipboardList : r.kind === 'html' ? Code2 : FileText
	}

	function href(r: ArtifactRow): string {
		return home ? artifactPath(home, r.id) : '#'
	}

	function trashHref(on: boolean): string {
		const url = new URL(page.url)
		if (on) url.searchParams.set('view', 'trash')
		else url.searchParams.delete('view')
		return url.pathname + url.search
	}

	async function share(r: ArtifactRow) {
		try {
			await shareDialog?.share({ id: r.id, name: r.name, local: r.local, path: r.path })
		} catch (err) {
			sendUserToast('Could not share the artifact', true, [], apiErrorMessage(err))
		}
	}

	let busy: string | undefined = $state()
	async function restore(r: ArtifactRow) {
		if (!home) return
		busy = r.id
		try {
			await restoreArtifact(r.id, home)
			logFeatureUsage('ai_artifacts', 'restore', { key: 'trash' })
			sendUserToast(`Restored "${r.name}"`)
			reloads++
		} catch (err) {
			sendUserToast(`Could not restore "${r.name}"`, true, [], apiErrorMessage(err))
		} finally {
			busy = undefined
		}
	}

	async function deleteForever(r: ArtifactRow) {
		if (!home) return
		busy = r.id
		try {
			await deleteArtifactForever(r.id, home)
			sendUserToast(`Deleted "${r.name}"`)
			reloads++
		} catch (err) {
			sendUserToast(`Could not delete "${r.name}"`, true, [], apiErrorMessage(err))
		} finally {
			busy = undefined
		}
	}
</script>

{#snippet audienceChip(who: { audience: Audience; label: string }, pinned: number | undefined)}
	{#if who.audience === 'only_you'}
		<Lock size={12} class="shrink-0" />
	{/if}
	<span class="truncate">{who.label}</span>
	{#if pinned !== undefined}
		<span class="shrink-0 text-hint">· v{pinned} pinned</span>
	{/if}
{/snippet}

<CenteredPage>
	<PageHeader
		title={trashView ? 'Recently deleted' : 'Artifacts'}
		tooltip={trashView
			? 'Artifacts you deleted, kept for 30 days before they are deleted for good.'
			: 'The documents the AI writes in your sessions: plans, reports and pages. Share one to give people in the workspace access to it.'}
	>
		{#if trashView}
			<Button
				variant="default"
				unifiedSize="md"
				href={trashHref(false)}
				startIcon={{ icon: ArrowLeft }}
			>
				Back to artifacts
			</Button>
		{:else}
			<Button variant="subtle" unifiedSize="md" href={trashHref(true)} startIcon={{ icon: Trash2 }}>
				Recently deleted
			</Button>
		{/if}
	</PageHeader>

	<div class="flex flex-col gap-4 pt-6 pb-20">
		{#if !trashView}
			<Tabs bind:selected={tab}>
				<Tab value="all" label="All" />
				<Tab value="mine" label="Created by me" />
				<Tab value="shared" label="Shared with me" />
			</Tabs>
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			<TextInput
				size="md"
				class="w-80 max-w-full"
				bind:value={search}
				inputProps={{
					type: 'search',
					placeholder: 'Search names and content',
					'aria-label': 'Search artifacts'
				}}
			/>
			<Select
				class="w-40"
				size="md"
				placeholder="Any kind"
				clearable
				items={[
					{ label: 'Markdown', value: 'md' },
					{ label: 'HTML', value: 'html' },
					{ label: 'Plan', value: 'plan' }
				]}
				bind:value={kind}
			/>
			{#if !trashView}
				<Select
					class="w-56"
					size="md"
					placeholder="Anyone can see it"
					clearable
					items={[
						{ label: 'Only you', value: 'only_you' },
						{ label: 'Everyone in the workspace', value: 'everyone' },
						{ label: 'Specific users and groups', value: 'specific' }
					]}
					bind:value={audience}
				/>
				{#if tab !== 'mine'}
					<Select
						class="w-44"
						size="md"
						placeholder="Any owner"
						clearable
						items={[
							...(tab === 'all' ? [{ label: 'You', value: ME }] : []),
							...owners.map((o) => ({ label: o, value: o }))
						]}
						bind:value={owner}
					/>
				{/if}
			{/if}
			<div class="grow"></div>
			<Select
				class="w-48"
				size="md"
				items={[
					{ label: 'Last updated', value: 'updated' },
					{ label: 'Last opened by me', value: 'opened' },
					{ label: 'Name', value: 'name' }
				]}
				bind:value={sort}
			/>
		</div>

		{#if loading}
			<Skeleton layout={[[2], 0.5, [2], 0.5, [2], 0.5, [2]]} />
		{:else if shown.length === 0}
			{#if listError}
				<EmptyState
					icon={AlertTriangle}
					title="Could not list artifacts from the server"
					description={listError}
					action={{
						label: 'Retry',
						icon: RefreshCw,
						onClick: () => reloads++,
						disabled: server.loading
					}}
				/>
			{:else if query}
				<EmptyState icon={Search} title="No artifact matches ‘{query}’." />
			{:else if filtering}
				<EmptyState icon={Search} title="No artifact matches these filters." />
			{:else if trashView}
				<EmptyState
					icon={Trash2}
					title="Nothing in Recently deleted"
					description="Artifacts you delete stay here for 30 days, then they are deleted for good."
				/>
			{:else if tab === 'shared'}
				<EmptyState
					icon={Users}
					title="Nothing is shared with you yet"
					description="Artifacts people share with you, with your groups or with the whole workspace show up here."
				/>
			{:else}
				<EmptyState
					icon={Files}
					title="No artifacts yet"
					description="Artifacts the AI writes in your sessions show up here."
					action={{
						label: 'Start a session',
						icon: MessageSquarePlus,
						onClick: () => void openNewSession(home)
					}}
				/>
			{/if}
		{:else}
			{#if listError}
				<Alert
					type="error"
					size="xs"
					title="Could not list artifacts from the server"
					actions={[
						{
							label: 'Retry',
							startIcon: { icon: RefreshCw },
							onClick: () => reloads++,
							loading: server.loading
						}
					]}
				>
					Only the artifacts this browser holds are listed. {listError}
				</Alert>
			{/if}
			<DataTable size="sm" tableFixed>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						{#if trashView}
							<Cell head class="w-48">Deleted</Cell>
							<Cell head last actions class="w-56"></Cell>
						{:else}
							<Cell head class="w-56">Who can see it</Cell>
							<Cell head class="w-32">Owner</Cell>
							<Cell head class="w-32">Updated</Cell>
							<Cell head last class="w-48">Session</Cell>
						{/if}
					</tr>
				</Head>
				<tbody class="divide-y">
					{#each groups as group (group.label ?? '')}
						{#if group.label}
							<tr>
								<td
									colspan={trashView ? 3 : 5}
									class="px-2 pt-3 pb-1 text-2xs font-semibold text-secondary"
								>
									{group.label}
								</td>
							</tr>
						{/if}
						{#each group.rows as r (r.id)}
							{@const Icon = rowIcon(r)}
							{@const who = audienceOf(r)}
							{@const session = sessionOf(r)}
							<Row hoverable={!trashView} on:click={() => !trashView && goto(href(r))}>
								<Cell first>
									<div class="flex min-w-0 flex-col gap-0.5">
										<div class="flex min-w-0 items-center gap-1.5">
											<Icon size={14} class="shrink-0 text-secondary" />
											{#if trashView}
												<span class="truncate text-xs font-semibold text-emphasis" title={r.name}>
													{r.name}
												</span>
											{:else}
												<!-- The row navigates too; the link is for a new tab and the keyboard. -->
												<a
													href={href(r)}
													class="truncate text-xs font-semibold text-emphasis hover:underline"
													title={r.name}
													onclick={(e) => e.stopPropagation()}
												>
													{r.name}
												</a>
											{/if}
										</div>
										{#if r.matchLine}
											<span class="truncate text-2xs text-secondary" title={r.matchLine}>
												{r.matchLine}
											</span>
										{/if}
									</div>
								</Cell>
								{#if trashView}
									<Cell class="text-xs text-secondary">
										{r.deletedAt ? displayDate(new Date(r.deletedAt)) : ''}
									</Cell>
									<Cell last actions>
										<div class="flex justify-end gap-1">
											<Button
												variant="default"
												unifiedSize="sm"
												startIcon={{ icon: RotateCcw }}
												disabled={busy === r.id}
												onclick={() => restore(r)}
											>
												Restore
											</Button>
											<Button
												variant="subtle"
												destructive
												unifiedSize="sm"
												disabled={busy === r.id}
												onclick={() => deleteForever(r)}
											>
												Delete forever
											</Button>
										</div>
									</Cell>
								{:else}
									<Cell shouldStopPropagation={canShare(r)}>
										{#if canShare(r)}
											<div class="-my-1 -ml-1 flex min-w-0">
												<Button
													variant="subtle"
													unifiedSize="xs"
													wrapperClasses="min-w-0"
													btnClasses="min-w-0 gap-1 text-secondary hover:text-primary"
													title="Change who can see it"
													onclick={() => share(r)}
												>
													{@render audienceChip(who, r.pinnedVersion)}
												</Button>
											</div>
										{:else}
											<span class="flex min-w-0 items-center gap-1 text-xs text-secondary">
												{@render audienceChip(who, r.pinnedVersion)}
											</span>
										{/if}
									</Cell>
									<Cell class="truncate text-xs text-secondary">
										{r.mine ? 'You' : `${r.owner}${r.ownerIsMember ? '' : ' (left)'}`}
									</Cell>
									<Cell class="text-xs text-secondary">
										<span class="tabular-nums">v{r.version}</span> ·
										<TimeAgo date={new Date(r.updatedAt).toISOString()} compact />
									</Cell>
									<Cell last class="truncate text-xs" shouldStopPropagation={!!session}>
										{#if session}
											<a
												class="text-secondary hover:text-primary hover:underline"
												href={withWorkspaceParam(
													sessionPageHref(session.id),
													session.workspace_id ?? session.pending_workspace_id
												)}
												title={session.summary ?? 'Untitled session'}
											>
												{session.summary ?? 'Untitled session'}
											</a>
										{:else if sessionDeleted(r)}
											<span class="text-hint">Session deleted</span>
										{/if}
									</Cell>
								{/if}
							</Row>
						{/each}
					{/each}
				</tbody>
			</DataTable>
		{/if}
	</div>
</CenteredPage>

<ArtifactShareDialog bind:this={shareDialog} workspace={home} onChange={() => reloads++} />

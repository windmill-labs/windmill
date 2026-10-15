<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { Globe, Link2Off, LogIn, MessagesSquare, Users } from 'lucide-svelte'
	import { Badge, Button, EmptyState, Skeleton } from '$lib/components/common'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import SharedSessionTranscript from '$lib/components/sessions/SharedSessionTranscript.svelte'
	import { readSessionSnapshot } from '$lib/components/sessions/sessionSharing'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { AiService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate } from '$lib/utils'

	// Reached without signing in when the share is public, so this page sets up what the
	// logged-in layout otherwise would.
	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	document.documentElement.classList.toggle('dark', darkMode === 'dark')
	applyDarkModeVariant()

	const workspace = $derived(page.params.workspace ?? '')
	const id = $derived(page.params.id ?? '')

	$effect(() => {
		$workspaceStore = workspace
	})

	type Loaded =
		| {
				state: 'found'
				session: Awaited<ReturnType<typeof AiService.getSharedAiSession>>
				snapshot: ReturnType<typeof readSessionSnapshot>
		  }
		| { state: 'gone' }
		| { state: 'signin' }
		| { state: 'error'; message: string }

	const shared = resource(
		() => ({ workspace, id }),
		async ({ workspace, id }): Promise<Loaded | undefined> => {
			if (!workspace || !id) return undefined
			try {
				const session = await AiService.getSharedAiSession({ workspace, id })
				return { state: 'found', session, snapshot: readSessionSnapshot(session.snapshot) }
			} catch (err) {
				const status = (err as { status?: number })?.status
				if (status === 401) return { state: 'signin' }
				// A malformed id is a 400 from the path extractor, and means the same to the reader.
				if (status === 404 || status === 400) return { state: 'gone' }
				return { state: 'error', message: String((err as { body?: unknown })?.body ?? err) }
			}
		}
	)

	const found = $derived(shared.current?.state === 'found' ? shared.current : undefined)
	const loginHref = $derived(
		`/user/login?rd=${encodeURIComponent(page.url.pathname + page.url.search)}`
	)

	let removing = $state(false)
	async function stopSharing() {
		if (!found) return
		removing = true
		try {
			await AiService.unshareAiSession({ workspace, id: found.session.id })
			shared.mutate({ state: 'gone' })
			sendUserToast('Stopped sharing: the link no longer opens')
		} catch (err) {
			sendUserToast(`Could not stop sharing: ${(err as { body?: unknown })?.body ?? err}`, true)
		} finally {
			removing = false
		}
	}
</script>

<!-- The transcript is its author's text, rendered to whoever holds the link: an image in it
     that points elsewhere would report every reader to that host. A policy stays with the
     document once set, so every link out of this page is a full load (`data-sveltekit-reload`). -->
<svelte:head>
	<meta http-equiv="content-security-policy" content="img-src 'self' data: blob:" />
</svelte:head>

<div class="flex min-h-screen flex-col bg-surface" data-sveltekit-reload>
	<div class="border-b bg-surface-secondary">
		<div class="mx-auto flex w-full max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
			<WindmillIcon height="20px" width="20px" />
			{#if found}
				<div class="flex min-w-0 flex-1 flex-col">
					<div class="flex min-w-0 items-center gap-2">
						<h1 class="truncate text-sm font-semibold text-emphasis" title={found.session.name}>
							{found.session.name}
						</h1>
						{#if found.session.is_public}
							<Badge color="blue" icon={{ icon: Globe, position: 'left' }}>Public</Badge>
						{:else}
							<Badge color="gray" icon={{ icon: Users, position: 'left' }}>{workspace}</Badge>
						{/if}
					</div>
					<span class="text-xs font-normal text-secondary">
						Read-only AI session shared by {found.session.created_by} · {displayDate(
							found.session.shared_at
						)}
					</span>
				</div>
				{#if found.session.can_unshare}
					<Button
						unifiedSize="sm"
						variant="default"
						destructive
						loading={removing}
						onClick={stopSharing}
					>
						Stop sharing
					</Button>
				{/if}
			{:else}
				<span class="text-sm font-semibold text-emphasis">Shared AI session</span>
			{/if}
		</div>
	</div>

	<div class="mx-auto w-full max-w-3xl flex-1 px-4 py-6">
		{#if found}
			{#if found.snapshot.messages.length > 0}
				{#key found.session.id}
					<SharedSessionTranscript snapshot={found.snapshot} {workspace} />
				{/key}
			{:else}
				<EmptyState icon={MessagesSquare} title="This session has no messages" />
			{/if}
		{:else if shared.current?.state === 'signin'}
			<EmptyState
				icon={LogIn}
				title="Sign in to read this session"
				description="It is shared with the members of {workspace} only."
			>
				<Button variant="accent" unifiedSize="md" href={loginHref}>Sign in</Button>
			</EmptyState>
		{:else if shared.current?.state === 'gone'}
			<EmptyState
				icon={Link2Off}
				title="This shared session is not available"
				description="Its author stopped sharing it, or it is shared with another workspace's members only."
			/>
		{:else if shared.current?.state === 'error'}
			<EmptyState
				icon={Link2Off}
				title="Could not load this shared session"
				description={shared.current.message}
			/>
		{:else}
			<Skeleton layout={[[3], 1, [30]]} />
		{/if}
	</div>
</div>

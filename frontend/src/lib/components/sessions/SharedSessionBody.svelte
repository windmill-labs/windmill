<script lang="ts">
	import { Link2Off, Loader2, Lock, MessagesSquare } from 'lucide-svelte'
	import { EmptyState } from '$lib/components/common'
	import { UserService } from '$lib/gen'
	import { resource } from 'runed'
	import { displayDate } from '$lib/utils'
	import type { Snippet } from 'svelte'
	import SharedSessionTranscript from './SharedSessionTranscript.svelte'
	import type { LoadedSharedSession } from './sharedSession.svelte'

	interface Props {
		loaded: LoadedSharedSession | undefined
		workspace: string
		/** The reader is signed in to the workspace, so the author's drafts may open for them. */
		member?: boolean
		/** Set when the reader has no session: signing in as a member opens the drafts. */
		signInHref?: string
		/** See SharedSessionTranscript. */
		previewActions?: Snippet
	}

	let { loaded, workspace, member = false, signInHref, previewActions }: Props = $props()

	const editsWorkspace = $derived(
		loaded?.state === 'found' ? loaded.snapshot.editsWorkspace : undefined
	)
	// A fork session's edits live in the fork while the share lives in its parent, so being a
	// member of the share's workspace says nothing about reaching them: the fork is asked.
	const forkMember = resource(
		() => (editsWorkspace && editsWorkspace !== workspace ? editsWorkspace : undefined),
		(fork) =>
			fork
				? UserService.whoami({ workspace: fork }).then(
						() => true,
						() => false
					)
				: Promise.resolve(false)
	)
	const inFork = $derived(!!editsWorkspace && editsWorkspace !== workspace)
	// A deployed item opens only inside its workspace, so a public share's non-member reader gets
	// it listed but locked, as for drafts.
	const opensEdits = $derived(inFork ? forkMember.current === true : member)
	const readsDrafts = $derived(inFork ? forkMember.current === true : member)
	const draftsLockedReason = $derived.by(() => {
		if (signInHref)
			return `You are signed out, and only members of ${editsWorkspace ?? workspace} can open the author's edits.`
		if (inFork) {
			return forkMember.current === false
				? `Edited in the fork ${editsWorkspace}, which you are not a member of: only its members can open the author's edits.`
				: undefined
		}
		return member ? undefined : `Only members of ${workspace} can open the author's edits.`
	})
</script>

{#snippet sharedBy()}
	{#if loaded?.state === 'found'}
		<div class="pt-2 text-center text-xs text-secondary">
			Read-only {loaded.session.is_public ? 'public ' : ''}copy shared by {loaded.session
				.created_by}
			· {displayDate(loaded.session.shared_at)}
		</div>
	{/if}
{/snippet}

{#if loaded?.state === 'found' && loaded.snapshot.messages.length > 0}
	{#key loaded.session.id}
		<SharedSessionTranscript
			snapshot={loaded.snapshot}
			{workspace}
			author={loaded.session.created_by}
			editsWorkspace={opensEdits ? editsWorkspace : undefined}
			{readsDrafts}
			{draftsLockedReason}
			draftsSignInHref={signInHref}
			footer={sharedBy}
			{previewActions}
		/>
	{/key}
{:else}
	<div class="h-full overflow-y-auto">
		<div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-7 pt-4">
			{@render placeholder()}
		</div>
	</div>
{/if}

{#snippet placeholder()}
	{#if loaded?.state === 'found'}
		<EmptyState icon={MessagesSquare} title="This session has no messages" />
		{@render sharedBy()}
	{:else if loaded?.state === 'gone'}
		<EmptyState
			icon={Link2Off}
			title="This shared session is not available"
			description="Its author stopped sharing it, or the link is incomplete."
		/>
	{:else if loaded?.state === 'outsider'}
		<EmptyState
			icon={Lock}
			title="Only members of {workspace} can open this link"
			description="You are signed in as {loaded.email}, who is not a member of {workspace}. Ask an admin of {workspace} to add you, or ask the author for a public link."
		/>
	{:else if loaded?.state === 'error'}
		<EmptyState
			icon={Link2Off}
			title="Could not load this shared session"
			description={loaded.message}
		/>
	{:else}
		<div class="flex flex-1 items-center justify-center py-16 text-tertiary">
			<Loader2 class="animate-spin" />
		</div>
	{/if}
{/snippet}

<script lang="ts">
	import { Link2Off, Loader2, Lock, MessagesSquare } from 'lucide-svelte'
	import { EmptyState } from '$lib/components/common'
	import { UserService } from '$lib/gen'
	import { resource } from 'runed'
	import SharedSessionTranscript from './SharedSessionTranscript.svelte'
	import type { LoadedSharedSession } from './sharedSession.svelte'

	interface Props {
		loaded: LoadedSharedSession | undefined
		workspace: string
		/** The reader is signed in to the workspace, so the author's drafts may open for them. */
		member?: boolean
	}

	let { loaded, workspace, member = false }: Props = $props()

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
	const opensEdits = $derived(inFork ? forkMember.current === true : true)
	const readsDrafts = $derived(inFork ? forkMember.current === true : member)
	const draftsLockedReason = $derived.by(() => {
		if (inFork) {
			return forkMember.current === false
				? `Edited in the fork ${editsWorkspace}, which you are not a member of: only its members can open the author's drafts.`
				: undefined
		}
		return member ? undefined : `Only members of ${workspace} can open the author's drafts.`
	})
</script>

{#if loaded?.state === 'found'}
	{#if loaded.snapshot.messages.length > 0}
		{#key loaded.session.id}
			<SharedSessionTranscript
				snapshot={loaded.snapshot}
				{workspace}
				author={loaded.session.created_by}
				editsWorkspace={opensEdits ? editsWorkspace : undefined}
				{readsDrafts}
				{draftsLockedReason}
			/>
		{/key}
	{:else}
		<EmptyState icon={MessagesSquare} title="This session has no messages" />
	{/if}
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

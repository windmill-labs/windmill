<script lang="ts">
	import { Link2Off, Loader2, MessagesSquare } from 'lucide-svelte'
	import { EmptyState } from '$lib/components/common'
	import SharedSessionTranscript from './SharedSessionTranscript.svelte'
	import type { LoadedSharedSession } from './sharedSession.svelte'

	interface Props {
		loaded: LoadedSharedSession | undefined
		workspace: string
		/** The reader is signed in to the workspace, so the author's drafts may open for them. */
		member?: boolean
	}

	let { loaded, workspace, member = false }: Props = $props()
</script>

{#if loaded?.state === 'found'}
	{#if loaded.snapshot.messages.length > 0}
		{#key loaded.session.id}
			<SharedSessionTranscript
				snapshot={loaded.snapshot}
				{workspace}
				draftAuthor={member ? loaded.session.created_by : undefined}
			/>
		{/key}
	{:else}
		<EmptyState icon={MessagesSquare} title="This session has no messages" />
	{/if}
{:else if loaded?.state === 'gone'}
	<EmptyState
		icon={Link2Off}
		title="This shared session is not available"
		description="Its author stopped sharing it, or it is shared with another workspace's members only."
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

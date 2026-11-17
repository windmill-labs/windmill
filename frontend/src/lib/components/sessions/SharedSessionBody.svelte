<script lang="ts">
	import { page } from '$app/state'
	import { Link2Off, LogIn, MessagesSquare } from 'lucide-svelte'
	import { Button, EmptyState, Skeleton } from '$lib/components/common'
	import SharedSessionTranscript from './SharedSessionTranscript.svelte'
	import type { LoadedSharedSession } from './sharedSession.svelte'

	interface Props {
		loaded: LoadedSharedSession | undefined
		workspace: string
		/** The reader is signed in to the workspace, so the author's drafts may open for them. */
		member?: boolean
	}

	let { loaded, workspace, member = false }: Props = $props()

	const loginHref = $derived(
		`/user/login?rd=${encodeURIComponent(page.url.pathname + page.url.search)}`
	)
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
{:else if loaded?.state === 'signin'}
	<EmptyState
		icon={LogIn}
		title="Sign in to read this session"
		description="It is shared with the members of {workspace} only."
	>
		<Button variant="accent" unifiedSize="md" href={loginHref}>Sign in</Button>
	</EmptyState>
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
	<Skeleton layout={[[3], 1, [30]]} />
{/if}

<script lang="ts">
	import { Button } from '$lib/components/common'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import SessionStatusPopover from '$lib/components/sessions/SessionStatusPopover.svelte'
	import { Download, Trash2 } from 'lucide-svelte'
	import { download, displayDate } from '$lib/utils'
	import { getAiChatManager } from '../aiChatManagerContext'
	import {
		artifactFilename,
		artifactMimeType,
		currentVersion,
		type PersistedArtifact
	} from './artifactsDB'

	const aiChatManager = getAiChatManager()
	const artifacts = $derived(aiChatManager.artifacts.artifacts)
	const label = $derived(`${artifacts.length} artifact${artifacts.length === 1 ? '' : 's'}`)

	// Empty-at-0 gating is owned by the parent (SessionChangesBar) so the status
	// line's separators stay correct; this renders unconditionally.
	let open = $state(false)
</script>

<SessionStatusPopover
	bind:open
	{label}
	title="Artifacts this session"
	items={artifacts}
	itemKey={(a) => a.id}
	rowTitle={(a) => a.name}
	onPick={(a: PersistedArtifact) => aiChatManager.openArtifact?.(a.id, a.name)}
>
	{#snippet row(a)}
		<span class="min-w-0 flex-1 truncate font-normal text-primary">{a.name}</span>
		<span
			class="shrink-0 rounded bg-surface-secondary px-1 py-0.5 text-2xs font-normal uppercase text-tertiary"
		>
			{a.kind}
		</span>
		<!-- The version rides in the timestamp's reserved width, paid for by the compact time:
		     the kind badge keeps its slot, and the name loses none of its own. -->
		<span
			class="min-w-[4.5rem] shrink-0 text-right text-2xs font-normal text-hint"
			title={displayDate(new Date(a.updatedAt))}
		>
			{#if currentVersion(a) > 1}<span class="tabular-nums">v{currentVersion(a)}</span> ·{/if}
			<TimeAgo date={new Date(a.updatedAt).toISOString()} compact />
		</span>
	{/snippet}
	{#snippet actions(a)}
		<Button
			unifiedSize="xs"
			variant="subtle"
			iconOnly
			title="Download"
			startIcon={{ icon: Download }}
			onClick={() => download(artifactFilename(a), a.content, artifactMimeType(a.kind))}
		/>
		<Button
			unifiedSize="xs"
			destructive
			variant="subtle"
			iconOnly
			title="Delete"
			startIcon={{ icon: Trash2 }}
			onClick={() => {
				aiChatManager.closeArtifact?.(a.id)
				void aiChatManager.artifacts.remove(a.id)
			}}
		/>
	{/snippet}
</SessionStatusPopover>

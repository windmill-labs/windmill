<script lang="ts">
	import { Button } from '$lib/components/common'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import SessionStatusPopover from '$lib/components/sessions/SessionStatusPopover.svelte'
	import { Download, Trash2 } from 'lucide-svelte'
	import { download, displayDate } from '$lib/utils'
	import { getAiChatManager } from '../aiChatManagerContext'
	import { artifactFilename, artifactMimeType, type PersistedArtifact } from './artifactsDB'

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
		<span
			class="min-w-[4.5rem] shrink-0 text-right text-2xs font-normal text-hint"
			title={displayDate(new Date(a.updatedAt))}
		>
			<TimeAgo date={new Date(a.updatedAt).toISOString()} noSeconds />
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

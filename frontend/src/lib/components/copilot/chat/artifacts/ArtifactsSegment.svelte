<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import SessionStatusToken, {
		TOKEN_TRIGGER_CLASS
	} from '$lib/components/sessions/SessionStatusToken.svelte'
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

	function openArtifact(a: PersistedArtifact) {
		aiChatManager.openArtifact?.(a.id, a.name)
		open = false
	}
</script>

<Popover
	bind:isOpen={open}
	placement="top-start"
	enableFlyTransition
	closeOnOtherPopoverOpen
	class={TOKEN_TRIGGER_CLASS}
	contentClasses="!bg-surface"
	triggerAttrs={{ 'aria-label': label, 'aria-haspopup': 'dialog' }}
>
	{#snippet trigger()}
		<SessionStatusToken {label} expanded={open} />
	{/snippet}

	{#snippet content()}
		<div class="flex w-80 flex-col text-xs">
			<div class="border-b px-3 py-2 text-tertiary">Artifacts this session</div>
			<div role="list" class="max-h-[min(10rem,50vh)] overflow-y-auto py-1">
				{#each artifacts as a (a.id)}
					<div
						class="flex items-center gap-1 py-1 pl-3 pr-1 hover:bg-surface-hover"
						role="listitem"
					>
						<button
							type="button"
							class="flex min-w-0 flex-1 items-center gap-2 text-left"
							title={a.name}
							onclick={() => openArtifact(a)}
						>
							<span class="min-w-0 flex-1 truncate font-normal text-secondary">{a.name}</span>
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
						</button>
						<Button
							size="xs"
							variant="subtle"
							iconOnly
							title="Download"
							startIcon={{ icon: Download }}
							onClick={() => download(artifactFilename(a), a.content, artifactMimeType(a.kind))}
						/>
						<Button
							size="xs"
							color="red"
							variant="subtle"
							iconOnly
							title="Delete"
							startIcon={{ icon: Trash2 }}
							onClick={() => {
								aiChatManager.closeArtifact?.(a.id)
								void aiChatManager.artifacts.remove(a.id)
							}}
						/>
					</div>
				{/each}
			</div>
		</div>
	{/snippet}
</Popover>

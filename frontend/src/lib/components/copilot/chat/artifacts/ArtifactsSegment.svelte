<script lang="ts">
	import { Button } from '$lib/components/common'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import SessionStatusPopover from '$lib/components/sessions/SessionStatusPopover.svelte'
	import { ClipboardList, Code2, FileText, Trash2 } from 'lucide-svelte'
	import { displayDate } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { getAiChatManager } from '../aiChatManagerContext'
	import { planBadge, listBadge, listOpenTarget, PLAN_MODE_TEXT_COLOR } from '../planMode'
	import { currentVersion, type PersistedArtifact } from './artifactsDB'
	import { planFirst } from './artifactsState.svelte'

	const aiChatManager = getAiChatManager()
	const artifacts = $derived(aiChatManager.artifacts.artifacts)
	// The plan is what the user comes back to, and in update order it sinks under
	// everything the approved run then produces.
	const planCount = $derived(artifacts.filter((a) => a.role === 'plan').length)
	const orderedArtifacts = $derived(planFirst(artifacts))
	const label = $derived(`${artifacts.length} artifact${artifacts.length === 1 ? '' : 's'}`)
	const rowIcon = (a: PersistedArtifact) =>
		a.role === 'plan' ? ClipboardList : a.kind === 'html' ? Code2 : FileText

	// Empty-at-0 gating is owned by the parent (SessionChangesBar) so the status
	// line's separators stay correct; this renders unconditionally.
	let open = $state(false)
</script>

<SessionStatusPopover
	bind:open
	{label}
	title="Artifacts this session"
	items={orderedArtifacts}
	itemKey={(a) => a.id}
	rowTitle={(a) => a.name}
	separatorAfter={(_, index) => index === planCount - 1 && planCount < orderedArtifacts.length}
	onPick={(a: PersistedArtifact) => aiChatManager.openArtifact?.(a.id, a.name, listOpenTarget(a))}
>
	{#snippet row(a)}
		{@const Icon = rowIcon(a)}
		<Icon
			class={twMerge('h-3 w-3 shrink-0', a.role === 'plan' ? PLAN_MODE_TEXT_COLOR : 'text-hint')}
		/>
		<span
			class={twMerge(
				'min-w-0 flex-1 truncate text-primary',
				a.role === 'plan' ? 'font-medium' : 'font-normal'
			)}>{a.name}</span
		>
		{@const badge = planBadge(listBadge(a))}
		<span
			class={twMerge(
				'shrink-0 rounded px-1 py-0.5 text-2xs uppercase',
				badge?.class ?? 'bg-surface-secondary font-normal text-tertiary'
			)}
		>
			{badge?.label ?? a.kind}
		</span>
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

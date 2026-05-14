<!--
@component
Count chip + popover dropdown listing modified workspace items between a
fork and its parent. Each row is a `WorkspaceItemRow` that opens the
editor in a new tab; for kinds the session editor pane can host, a
"Open in side panel" affordance fades in on hover and calls
`onOpenInPanel`.

Pure presentational — caller supplies the diff list and the fork
workspace_id (used to build edit URLs).
-->
<script lang="ts">
	import { ExternalLink, PanelRightOpen } from 'lucide-svelte'
	import type { WorkspaceItemDiff } from '$lib/gen'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import WorkspaceItemRow from '$lib/components/WorkspaceItemRow.svelte'
	import { EDITOR_TARGET_KINDS, type SessionTarget } from './sessionState.svelte'
	import { editUrlFor } from './forkEditUrl'

	let {
		diffs,
		workspaceId,
		onOpenInPanel
	}: {
		diffs: WorkspaceItemDiff[]
		workspaceId: string
		onOpenInPanel?: (target: SessionTarget) => void
	} = $props()

	const totalDiffs = $derived(diffs.length)

	function diffToTarget(d: WorkspaceItemDiff): SessionTarget | undefined {
		if (!EDITOR_TARGET_KINDS.has(d.kind as SessionTarget['kind'])) return undefined
		return { kind: d.kind as SessionTarget['kind'], path: d.path }
	}

	function rowOnClickFallback(d: WorkspaceItemDiff) {
		// Anchor handles editor-supported kinds via href. Other kinds (no
		// editor) get no-op clicks; future work can wire deep-links here.
		const url = editUrlFor(d, workspaceId)
		if (url) window.open(url, '_blank', 'noopener,noreferrer')
	}
</script>

{#if totalDiffs > 0}
	<Popover placement="bottom-end" usePointerDownOutside disableFocusTrap class="inline-flex">
		{#snippet trigger()}
			<button
				type="button"
				title="{totalDiffs} modified item{totalDiffs === 1 ? '' : 's'}"
				class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-2xs font-medium text-secondary bg-surface hover:bg-surface-hover"
			>
				{totalDiffs} modified
			</button>
		{/snippet}
		{#snippet content({ close })}
			<div class="w-80 max-h-96 overflow-y-auto py-1 bg-surface rounded-md">
				{#each diffs as d (`${d.kind}/${d.path}`)}
					{@const target = diffToTarget(d)}
					{@const url = editUrlFor(d, workspaceId)}
					<WorkspaceItemRow
						kind={d.kind}
						summary={undefined}
						secondary={d.path}
						baseClass="py-1"
						href={url}
						onclick={url ? undefined : () => rowOnClickFallback(d)}
					>
						{#snippet extras()}
							{#if target}
								<button
									type="button"
									title="Open in side panel"
									aria-label="Open in side panel"
									class="opacity-0 group-hover:opacity-100 transition-opacity inline-flex items-center justify-center w-5 h-5 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
									onclick={(e) => {
										e.preventDefault()
										e.stopPropagation()
										onOpenInPanel?.(target)
										close()
									}}
								>
									<PanelRightOpen class="w-3.5 h-3.5" />
								</button>
							{/if}
							<ExternalLink class="w-3 h-3 text-tertiary shrink-0" />
						{/snippet}
					</WorkspaceItemRow>
				{/each}
			</div>
		{/snippet}
	</Popover>
{/if}
